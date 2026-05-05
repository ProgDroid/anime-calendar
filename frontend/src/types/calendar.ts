import type { Item } from './item'

export type EventStyle = 'timed' | 'all_day'

export interface Calendar {
  id: number;
  items: Item[];
  language: 'english' | 'romaji' | 'native';
  name: string;
  event_style?: EventStyle;
  created_at: string;
  updated_at: string;
}

export interface PageCalendar {
  id: number;
  item_count: number;
  airing_count: number;
  name: string;
  subscription_token: string;
  created_at: string;
  updated_at: string;
  recent_item_ids: number[];
}
