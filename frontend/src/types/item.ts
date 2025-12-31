export interface Title {
  english: string;
  native: string;
  romaji: string;
}

export interface CoverImage {
  extraLarge: string;
  large: string;
  medium: string;
  color: string;
}

export interface Item {
  id: number;
  id_mal: number | null;
  title: Title;
  media_type: 'ANIME' | 'MANGA';
  episode_duration: number;
  airing_schedule: any[];
  cover_image?: CoverImage;
  banner_image?: string;
}
