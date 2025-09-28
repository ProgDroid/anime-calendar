use chrono::NaiveDateTime;
use common::{calendar::Calendar, language::Language};
use icalendar::{Calendar as Ics, Component, Event, EventLike};
use log::info;

#[allow(deprecated, clippy::cast_possible_wrap)]
pub fn generate_calendar_export(calendar: &Calendar) -> Ics {
    let events: Vec<Event> = calendar
        .items
        .iter()
        .flat_map(|item| {
            info!("Processing item {}", item.title.english);

            let items: Vec<Event> = item
                .airing_schedule
                .iter()
                .map(|episode| {
                    info!(
                        "Processing {} episode {} airing at {}",
                        item.title.english,
                        episode.episode,
                        episode.airing_at.to_int()
                    );

                    Event::new()
                        .all_day(
                            NaiveDateTime::from_timestamp(episode.airing_at.to_int() as i64, 0)
                                .date(),
                        )
                        .summary(
                            format!(
                                "{} - Episode {}",
                                match calendar.language {
                                    Language::English => item.title.english.clone(),
                                    Language::Romaji => item.title.romaji.clone(),
                                    Language::Native => item.title.native.clone(),
                                },
                                episode.episode
                            )
                            .as_str(),
                        )
                        .to_owned()
                })
                .collect();

            items
        })
        .collect();

    let mut ics = Ics::new();
    let mut ics = ics.name(&calendar.name);

    for event in events {
        ics = ics.push(event);
    }

    ics.done()
}
