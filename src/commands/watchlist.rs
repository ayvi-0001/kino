use std::collections::HashSet;

use ::serenity::all::{CreateActionRow, CreateButton, EditMessage};
use poise::{CreateReply, serenity_prelude as serenity};
use tokio::sync::MutexGuard;

use crate::{
    db::Watchlist,
    diff::create_patch,
    state::{Context, Data, Error},
    utils::{message_link, now},
};

pub static MESSAGE_LIMIT: usize = 2000;
pub static MODAL_INPUT_LIMIT: usize = 4000;

/// manage this channels watch list
#[poise::command(
    slash_command,
    subcommands("create", "clear", "edit"),
    subcommand_required
)]
pub async fn watchlist(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// create pinned watch list message in this channel
#[poise::command(slash_command, guild_only)]
pub async fn create(
    ctx: Context<'_>,
    // #[description = "optional list name"] name: Option<String>,
) -> Result<(), Error> {
    let guild_id: serenity::GuildId = ctx.guild_id().expect("this command is set to `guild_only`");
    let channel_id: serenity::ChannelId = ctx.channel_id();
    let author_id: serenity::UserId = ctx.author().id;

    let permitted = ctx
        .author_member()
        .await
        .and_then(|member| member.permissions)
        .is_some_and(|perms| perms.manage_messages() || perms.administrator());

    if !permitted {
        let reply = poise::CreateReply::default()
            .content("you need the `Manage Messages` permission to set up the watch list.")
            .ephemeral(true);

        ctx.send(reply).await?;

        return Ok(());
    }

    let _guard: MutexGuard<'_, ()> = ctx.data().write_lock.lock().await;

    let existing = ctx.data().db.get_list(guild_id.get() as i64, channel_id.get() as i64).await?;
    if let Some(list) = existing {
        ctx.defer_ephemeral().await?;
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "a list already exists for this channel: {}.",
                    message_link(list.guild_id, list.channel_id, list.message_id)
                ))
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    }

    ctx.defer().await?;

    let message = ctx
        .say(
            WatchListContent::default()
                .updated_at(now())
                .updated_by(author_id.get() as i64)
                .render(),
        )
        .await?
        .into_message()
        .await?;

    if let Err(error) = message.pin(&ctx.http()).await {
        tracing::warn!(?error, "could not pin the watch list message");
    }

    ctx.data()
        .db
        .update_list(
            guild_id.get() as i64,
            channel_id.get() as i64,
            message.id.get() as i64,
            author_id.get() as i64,
            None,
        )
        .await?;

    Ok(())
}

/// clear this channels watch list
#[poise::command(slash_command, guild_only)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id: serenity::GuildId = ctx.guild_id().expect("this command is set to `guild_only`");
    let channel_id: serenity::ChannelId = ctx.channel_id();
    let author_id: serenity::UserId = ctx.author().id;

    let existing = ctx.data().db.get_list(guild_id.get() as i64, channel_id.get() as i64).await?;
    let Some(list) = existing else {
        ctx.send(CreateReply::default().content("no list exists in this channel").ephemeral(true))
            .await?;
        return Ok(());
    };

    let _guard: MutexGuard<'_, ()> = ctx.data().write_lock.lock().await;

    let confirm_button = CreateButton::new("confirm_yes")
        .label("confirm")
        .style(serenity::ButtonStyle::Success);
    let cancel_button = CreateButton::new("confirm_no")
        .label("cancel")
        .style(serenity::ButtonStyle::Danger);

    let action_row = CreateActionRow::Buttons(vec![confirm_button, cancel_button]);

    // TODO(ayvi0001): make confirmation ephermal

    let reply = CreateReply::default().content("are you sure?").components(vec![action_row]);

    ctx.send(reply).await?;

    while let Some(mci) = serenity::ComponentInteractionCollector::new(ctx).guild_id(guild_id)
        .author_id(author_id)
        .channel_id(channel_id)
        .timeout(std::time::Duration::from_secs(120))
        // .filter(move |mci| mci.data.custom_id == )
        .await
    {
        let mut msg = mci.message.clone();

        match mci.data.custom_id.as_str() {
            "confirm_yes" => {
                let content = format!("data: {:?}", mci.data);
                msg.edit(ctx, EditMessage::new().content(content).components(vec![])).await?;
                ctx.data().db.delete_all_list_entries(list.id).await?;
            }
            "confirm_no" => {
                msg.delete(&ctx.http()).await?;
            }
            _ => {
                mci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
            }
        }
    }

    Ok(())
}

#[derive(Debug, poise::Modal)]
#[name = "edit watch list"]
struct EditListModal {
    #[name = "movies (one entry per line)"]
    #[placeholder = "add, remove or reorder movies..."]
    #[paragraph]
    #[min_length = 0]
    #[max_length = 4000]
    content: String,
}

/// open the watch list in a modal editor, post resulting diff
#[poise::command(slash_command, guild_only)]
pub async fn edit(ctx: poise::ApplicationContext<'_, Data, Error>) -> Result<(), Error> {
    use poise::Modal as _;

    let guild_id: serenity::GuildId = ctx.guild_id().expect("this command is set to `guild_only`");
    let channel_id: serenity::ChannelId = ctx.channel_id();
    let author_id: serenity::UserId = ctx.author().id;

    let Some(list) = ctx.data().db.get_list(guild_id.get() as i64, channel_id.get() as i64).await?
    else {
        ctx.send(
            CreateReply::default()
                .content("there is no watch list yet. run `/watchlist create`")
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    };

    let entries = ctx.data().db.get_list_entries(list.id).await?;
    let mut content = entries.iter().map(|e| e.name.to_owned()).collect::<Vec<String>>().join("\n");
    content.push('\n');

    if content.len() > MODAL_INPUT_LIMIT {
        ctx.send(
            CreateReply::default()
                .content("watch list is too long to fit in the editor (i'm still working on this).")
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    }

    let Some(data) =
        EditListModal::execute_with_defaults(ctx, EditListModal { content: content.clone() })
            .await?
    else {
        ctx.send(CreateReply::default().content("failed to retrieve modal data").ephemeral(true))
            .await?;

        return Ok(());
    };

    let _guard: MutexGuard<'_, ()> = ctx.data().write_lock.lock().await;

    ctx.defer().await?;

    let base_revision: i64 = list.revision;

    let old_content: String = content.clone();
    let new_content: String = normalize(&data.content);

    if new_content == old_content {
        ctx.send(CreateReply::default().content("no changes made").ephemeral(true))
            .await?;

        return Ok(());
    }

    let old_entries: Vec<&str> = split_entries(&old_content);
    let new_entries: Vec<&str> = split_entries(&new_content);

    let new_set: HashSet<&str> = new_entries.iter().copied().collect();
    let removed: Vec<&str> = old_entries.iter().copied().filter(|s| !new_set.contains(s)).collect();

    ctx.data().db.delete_list_entries(list.id, &removed).await?;
    ctx.data()
        .db
        .update_list_entries(list.id, author_id.get() as i64, new_entries)
        .await?;
    ctx.data()
        .db
        .update_list(
            list.guild_id,
            list.channel_id,
            list.message_id,
            author_id.get() as i64,
            Some(&new_content),
        )
        .await?;

    let message_id: serenity::MessageId = sync_pinned(ctx, &list, &new_content).await?;

    let changes: String = create_patch(&old_content, &new_content, 1500);

    let mut response: String = format!(
        "@here: <@{}> updated the movie watch list in {}\n```diff\n{}```",
        author_id.get(),
        message_link(list.guild_id, list.channel_id, message_id.get() as i64),
        changes
    );

    if base_revision >= 0 && base_revision != list.revision {
        response
            .push_str("\n-# heads up, someone else edited the list while this editor was open.");
    }

    response.truncate(MESSAGE_LIMIT);

    ctx.send(CreateReply::default().content(response)).await?;

    Ok(())
}

async fn sync_pinned(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    list: &Watchlist,
    content: &str,
) -> Result<serenity::MessageId, Error> {
    let channel_id = serenity::ChannelId::new(list.channel_id as u64);
    let message_id = serenity::MessageId::new(list.message_id as u64);

    let entries: Vec<&str> = split_entries(content);

    let content: String = WatchListContent::default()
        .updated_at(list.updated_at)
        .updated_by(list.author_id)
        .entries(entries)
        .render();

    let edit = serenity::EditMessage::new().content(&content);

    if channel_id.edit_message(&ctx.http(), message_id, edit).await.is_ok() {
        return Ok(message_id);
    }

    let message = ctx.send(CreateReply::default().content(&content)).await?.into_message().await?;

    if let Err(error) = message.pin(&ctx.http()).await {
        tracing::warn!(?error, "could not pin the watch list message");
    }

    Ok(message.id)
}

fn strip_marker(line: &str) -> &str {
    let trimmed = line.trim();

    for marker in ["- ", "* ", "+ ", "• ", "– ", "— "] {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return rest.trim_start();
        }
    }

    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() && digits.len() <= 4 {
        let rest = &trimmed[digits.len()..];
        for separator in [". ", ") ", "- ", ".", ")"] {
            if let Some(rest) = rest.strip_prefix(separator) {
                return rest.trim_start();
            }
        }
    }

    trimmed
}

fn normalize(input: &str) -> String {
    let mut entries: Vec<String> = vec![];

    for line in input.replace('\r', "").lines() {
        let entry = strip_marker(line);
        if entry.is_empty() {
            continue;
        }
        let already_present = entries.iter().any(|existing| existing.eq_ignore_ascii_case(entry));
        if !already_present {
            entries.push(entry.to_owned());
        }
    }

    let mut content = entries.join("\n");
    content.push('\n');

    content
}

fn split_entries(content: &str) -> Vec<&str> {
    content.lines().filter(|line| !line.trim().is_empty()).collect()
}

#[derive(Default, Debug)]
struct WatchListContent {
    body: String,
    pub entries: Vec<String>,
    footer: String,
    header: String,
    pub updated_at: Option<i64>,
    pub updated_by: Option<i64>,
}

impl WatchListContent {
    pub fn entries(mut self, entries: Vec<&str>) -> Self {
        self.entries = entries.into_iter().map(String::from).collect::<Vec<String>>();
        self
    }
    pub fn updated_at(mut self, updated_at: i64) -> Self {
        self.updated_at = Some(updated_at);
        self
    }
    pub fn updated_by(mut self, updated_by: i64) -> Self {
        self.updated_by = Some(updated_by);
        self
    }

    pub fn render(mut self) -> String {
        self.write_content()
    }
}

impl WatchListContent {
    fn write_header(&mut self) {
        self.header = format!(
            "## Watch List\n-# {} {}\n\n",
            self.entries.len(),
            if self.entries.len() == 1 { "entry" } else { "entries" }
        );
    }

    fn write_footer(&mut self) {
        self.footer = String::from("\n-# ");

        if let Some(updated_by) = &self.updated_by {
            self.footer.push_str(&format!("last updated by <@{0}> ", updated_by));
        }
        if let Some(updated_at) = self.updated_at
            && updated_at > 0
        {
            self.footer.push_str(&format!("<t:{0}:R> ", updated_at));
        }

        self.footer.push_str("• edit with `/watchlist edit`");
    }

    fn write_body(&mut self) {
        let budget = MESSAGE_LIMIT.saturating_sub(self.header.len() + self.footer.len() + 40);

        let mut shown = 0_usize;
        for (idx, entry) in self.entries.iter().enumerate() {
            let line = format!("{}. {}\n", idx + 1, entry);

            if self.body.len() + line.len() > budget {
                break;
            }
            self.body.push_str(&line);
            shown += 1;
        }

        if shown < self.entries.len() {
            self.body.push_str(&format!("*…and {} more*\n", self.entries.len() - shown));
        }
    }

    fn write_content(&mut self) -> String {
        self.write_header();
        self.write_footer();

        if self.entries.is_empty() {
            format!("{0}*the list is empty.*{1}", self.header, self.footer)
        } else {
            self.write_body();
            format!("{}{}{}", self.header, self.body, self.footer)
        }
    }
}

