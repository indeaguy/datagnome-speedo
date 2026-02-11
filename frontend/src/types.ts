export type FeatureConfig = {
  enabled: boolean;
  custom_request: string;
};

export type NewsletterConfig = {
  id: string;
  user_id: string;
  title: string;
  topics: string[];
  tone: string;
  length: string;
  send_time_utc: string;
  timezone: string;
  delivery_email: string;
  is_active: boolean;
  features: Record<string, FeatureConfig>;
  created_at: string;
  updated_at: string;
};

export const DEFAULT_FEATURES: Record<string, FeatureConfig> = {
  kpis: { enabled: true, custom_request: '' },
  competitor_analysis: { enabled: false, custom_request: '' },
  market_segment_summary: { enabled: false, custom_request: '' },
  identify_risks: { enabled: false, custom_request: '' },
};
