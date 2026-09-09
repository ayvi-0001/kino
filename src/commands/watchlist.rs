crate::mod_flat!(pinned_message, entry_parts);
use std::{collections::HashSet, convert::Into};

use ::serenity::all::{CreateActionRow, CreateButton, EditMessage};
use poise::{CreateReply, serenity_prelude as serenity};
use tokio::sync::MutexGuard;

use self::pinned_message::WatchListPinnedMessage;
use crate::{
    diff::create_patch,
    state::{Context, Data, Error},
    utils::{message_link, now},
};

pub static MODAL_INPUT_LIMIT: usize = 4000;

/// manage this channels watch list
#[poise::command(
    slash_command,
    subcommands("create", "clear", "edit", "delete")
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
            WatchListPinnedMessage::default()
                .updated_at(now())
                .updated_by(author_id.get() as i64)
                .render_content(),
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

/// delete this channels watch list
/// NOTE: delete command currently set to prefix/owners only.
#[poise::command(
    prefix_command,
    guild_only,
    hide_in_help,
    owners_only
)]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id: serenity::GuildId = ctx.guild_id().expect("this command is set to `guild_only`");
    let channel_id: serenity::ChannelId = ctx.channel_id();

    // if let poise::Context::Prefix(prefix_ctx) = ctx {
    //     prefix_ctx.msg.delete(ctx).await?;
    // }

    let Some(list) = ctx.data().db.get_list(guild_id.get() as i64, channel_id.get() as i64).await?
    else {
        ctx.send(CreateReply::default().content("no list exists in this channel").ephemeral(true))
            .await?;
        return Ok(());
    };

    ctx.data().db.delete_list(list.id).await?;

    channel_id
        .delete_message(
            ctx,
            <u64 as Into<serenity::MessageId>>::into(list.message_id as u64),
        )
        .await?;

    ctx.send(CreateReply::default().content("channel watchlist deleted!")).await?;

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
pub async fn edit(
    ctx: poise::ApplicationContext<'_, Data, Error>,
    #[description = "notify all users with @everyone/@here. (default=true)"] notify: Option<bool>,
) -> Result<(), Error> {
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
    let new_content: String = WatchListPinnedMessage::normalize(&data.content);

    if new_content == old_content {
        ctx.send(CreateReply::default().content("no changes made").ephemeral(true))
            .await?;

        return Ok(());
    }

    let old_entries: Vec<&str> =
        old_content.lines().filter(|line| !line.trim().is_empty()).collect();
    let new_entries: Vec<&str> =
        new_content.lines().filter(|line| !line.trim().is_empty()).collect();

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

    let message_id: serenity::MessageId =
        WatchListPinnedMessage::sync_pinned(ctx, &list, &new_content).await?;

    let changes: String = create_patch(&old_content, &new_content, 1500);

    let should_notify: bool;
    if let Some(n) = notify {
        should_notify = n
    } else {
        should_notify = true;
    };

    let response: String = WatchListPinnedMessage::write_edit_reply(
        list.guild_id,
        list.channel_id,
        message_id.get() as i64,
        author_id.get() as i64,
        changes,
        base_revision,
        list.revision,
        should_notify,
    );

    ctx.send(CreateReply::default().content(response)).await?;

    Ok(())
}
