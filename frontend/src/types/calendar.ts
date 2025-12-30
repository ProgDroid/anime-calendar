import type { Item } from './item'

export interface Calendar {
  id: number;
  items: Item[];
  language: 'english' | 'romaji' | 'native';
  name: string;
  created_at: string;
  updated_at: string;
}
