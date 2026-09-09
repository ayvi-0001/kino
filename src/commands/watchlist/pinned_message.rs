use poise::{CreateReply, serenity_prelude as serenity};

use crate::{
    commands::watchlist::entry_parts::EntryParts,
    db::Watchlist,
    state::{Data, Error},
    utils::message_link,
};

pub static MESSAGE_LIMIT: usize = 2000;

#[derive(Default, Debug)]
pub(super) struct WatchListPinnedMessage {
    body: String,
    pub(super) entries: Vec<String>,
    footer: String,
    header: String,
    pub(super) updated_at: Option<i64>,
    pub(super) updated_by: Option<i64>,
}

impl WatchListPinnedMessage {
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

    pub fn render_content(mut self) -> String {
        self.write_content()
    }
}

// WatchListPinnedMessage assoociated methods
impl WatchListPinnedMessage {
    #[allow(clippy::too_many_arguments)]
    pub fn write_edit_reply(
        guild_id: i64,
        channel_id: i64,
        message_id: i64,
        author_id: i64,
        changes: String,
        base_revision: i64,
        latest_revision: i64,
        notify: bool,
    ) -> String {
        let mut response = format!(
            "{}<@{}> updated the movie watch list in {}\n```diff\n{}```",
            if notify { "@here: " } else { "" },
            author_id,
            message_link(guild_id, channel_id, message_id),
            changes
        );

        if base_revision >= 0 && base_revision != latest_revision {
            response.push_str(
                "\n-# heads up, someone else edited the list while this editor was open.",
            );
        };

        response.truncate(MESSAGE_LIMIT);

        response
    }

    pub async fn sync_pinned(
        ctx: poise::ApplicationContext<'_, Data, Error>,
        list: &Watchlist,
        content: &str,
    ) -> Result<serenity::MessageId, Error> {
        let channel_id = serenity::ChannelId::new(list.channel_id as u64);
        let message_id = serenity::MessageId::new(list.message_id as u64);

        let entries: Vec<&str> = content.lines().filter(|line| !line.trim().is_empty()).collect();

        let content: String = WatchListPinnedMessage::default()
            .updated_at(list.updated_at)
            .updated_by(list.author_id)
            .entries(entries)
            .render_content();

        let edit = serenity::EditMessage::new().content(&content).embeds(vec![]);

        if channel_id.edit_message(&ctx.http(), message_id, edit).await.is_ok() {
            return Ok(message_id);
        }

        let message =
            ctx.send(CreateReply::default().content(&content)).await?.into_message().await?;

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

    pub fn normalize(input: &str) -> String {
        let mut entries: Vec<String> = vec![];

        for line in input.replace('\r', "").lines() {
            let entry = WatchListPinnedMessage::strip_marker(line);
            if entry.is_empty() {
                continue;
            }
            let already_present =
                entries.iter().any(|existing| existing.eq_ignore_ascii_case(entry));
            if !already_present {
                entries.push(entry.to_owned());
            }
        }

        let mut content = entries.join("\n");
        content.push('\n');

        content
    }
}

impl WatchListPinnedMessage {
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

            // TODO(ayvi-0001): search movie on tmdb api
            if let Ok(EntryParts(name, year)) = EntryParts::new(entry) {
                tracing::info!("extracted entry parts: name={:#?}, year={:#?}", name, year);
            }

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
