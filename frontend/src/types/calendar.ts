import type { Item } from './item'

export type EventStyle = 'timed' | 'all_day'

export interface Calendar {
  id: number;
  user_id?: number;
  items: Item[];
  language: 'english' | 'romaji' | 'native';
  name: string;
  event_style?: EventStyle;
  created_at: string;
  updated_at: string;
  meta_version?: number;
}

/**
 * Owner projection on `shared_with_me` listing entries. `display` mirrors the
 * server's username (or future profile display name) and `avatar` is a URL or
 * `null` until a profile column lands. Treat both as opaque strings.
 */
export interface CalendarOwner {
  id: number;
  display: string;
  avatar: string | null;
}

/**
 * Listing entry for an *owned* calendar on `GET /calendars`.
 *
 * `editor_count` is the number of currently-active editors on this calendar
 * (always 0 today — the field surfaces the `+N editors` chip on owned cards
 * once Phase 2 editor mutations land).
 */
export interface PageCalendar {
  id: number;
  item_count: number;
  airing_count: number;
  editor_count: number;
  name: string;
  subscription_token: string;
  created_at: string;
  updated_at: string;
  recent_item_ids: number[];
}

/**
 * Listing entry for a *shared* calendar on `GET /calendars`.
 *
 * Mirrors `PageCalendar` minus `subscription_token` (editors don't get the
 * share link) and `editor_count`, plus an `owner` projection.
 */
export interface SharedPageCalendar {
  id: number;
  item_count: number;
  airing_count: number;
  name: string;
  created_at: string;
  updated_at: string;
  recent_item_ids: number[];
  owner: CalendarOwner;
}

export interface CalendarsPagination {
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
}

/**
 * Combined response for `GET /calendars`. `owned` keeps pagination; the
 * `shared_with_me` list is flat.
 */
export interface CalendarsResponse {
  owned: {
    data: PageCalendar[];
    pagination: CalendarsPagination;
  };
  shared_with_me: SharedPageCalendar[];
}
