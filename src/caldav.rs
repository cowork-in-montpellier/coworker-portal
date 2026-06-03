use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event, EventLike};

pub struct CalDavClient<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub calendar_id: &'a str,
}

impl<'a> CalDavClient<'a> {
    fn event_url(&self, uid: &str) -> String {
        let encoded_id = self.calendar_id.replace('@', "%40");
        format!(
            "https://www.google.com/calendar/dav/{}/events/{}.ics",
            encoded_id, uid
        )
    }

    pub async fn create_event(
        &self,
        uid: &str,
        summary: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        description: &str,
    ) -> anyhow::Result<()> {
        let event = Event::new()
            .summary(summary)
            .uid(uid)
            .starts(start)
            .ends(end)
            .description(description)
            .done();
        let mut calendar = Calendar::new();
        calendar.push(event);

        let resp = reqwest::Client::new()
            .put(self.event_url(uid))
            .basic_auth(self.email, Some(self.password))
            .header("Content-Type", "text/calendar; charset=utf-8")
            .body(calendar.to_string())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("CalDAV PUT failed: {} — {}", status, body);
        }
        Ok(())
    }

    pub async fn delete_event(&self, uid: &str) -> anyhow::Result<()> {
        let resp = reqwest::Client::new()
            .delete(self.event_url(uid))
            .basic_auth(self.email, Some(self.password))
            .send()
            .await?;

        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("CalDAV DELETE failed: {}", resp.status());
        }
        Ok(())
    }
}
