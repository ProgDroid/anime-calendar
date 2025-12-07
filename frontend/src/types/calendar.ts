import type { Item } from './item'

export interface Calendar {
  id: number;
  items: Item[];
  language: 'english' | 'romaji' | 'native';
  name: string;
}
