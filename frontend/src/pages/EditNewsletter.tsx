import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  getNewsletter,
  createNewsletter,
  updateNewsletter,
  sendNewsletterSample,
  type CreateNewsletterBody,
} from '../lib/api';
import { supabase } from '../supabaseClient';
import { DEFAULT_FEATURES } from '../types';

const TONES = ['neutral', 'playful', 'serious', 'professional'];
const LENGTHS = ['short', 'medium', 'long'];
const FEATURE_KEYS = ['kpis', 'competitor_analysis', 'market_segment_summary', 'identify_risks'] as const;
const FEATURE_LABELS: Record<string, string> = {
  kpis: 'KPIs',
  competitor_analysis: 'Competitor analysis',
  market_segment_summary: 'Market segment summary',
  identify_risks: 'Identify risks',
};
const FEATURE_DESCRIPTIONS: Record<string, string> = {
  kpis: 'Key performance indicators: metrics, trends, and how they compare to targets or prior period.',
  competitor_analysis: 'Summary of what competitors are doing and how they compare.',
  market_segment_summary: 'Overview of market segments, size, and how they are changing.',
  identify_risks: 'Risks and uncertainties that could affect your business or market.',
};
const FEATURE_PLACEHOLDERS: Record<string, string> = {
  kpis: 'e.g. Revenue, conversion, churn; compare to last quarter and highlight outliers.',
  competitor_analysis: 'e.g. Focus on pricing moves and who\'s gaining share in EMEA.',
  market_segment_summary: 'e.g. Break out by enterprise vs SMB and call out growth rates.',
  identify_risks: 'e.g. Include regulatory and supply chain, rank by likelihood.',
};

export function EditNewsletter() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(!!id);
  const [saving, setSaving] = useState(false);
  const [sendingSample, setSendingSample] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [form, setForm] = useState<CreateNewsletterBody>({
    title: '',
    topics: [],
    tone: 'neutral',
    length: 'medium',
    send_time_utc: '09:00',
    timezone: 'UTC',
    delivery_email: '',
    is_active: true,
    features: { ...DEFAULT_FEATURES },
  });

  useEffect(() => {
    if (!id) {
      setLoading(false);
      supabase.auth.getUser().then(({ data: { user } }) => {
        if (user?.email) {
          setForm((prev) => (prev.delivery_email ? prev : { ...prev, delivery_email: user.email ?? '' }));
        }
      });
      return;
    }
    getNewsletter(id)
      .then((c) => {
        setForm({
          title: c.title,
          topics: c.topics ?? [],
          tone: c.tone,
          length: c.length,
          send_time_utc: c.send_time_utc?.slice(0, 5) ?? '09:00',
          timezone: c.timezone,
          delivery_email: c.delivery_email,
          is_active: c.is_active,
          features: { ...DEFAULT_FEATURES, ...(c.features as Record<string, { enabled: boolean; custom_request: string }>) },
        });
      })
      .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load'))
      .finally(() => setLoading(false));
  }, [id]);

  const update = (patch: Partial<CreateNewsletterBody>) => {
    setForm((prev) => ({ ...prev, ...patch }));
  };

  const updateFeature = (key: string, enabled?: boolean, custom_request?: string) => {
    setForm((prev) => {
      const features = { ...(prev.features ?? {}) };
      const current = features[key] ?? { enabled: false, custom_request: '' };
      features[key] = {
        enabled: enabled ?? current.enabled,
        custom_request: custom_request ?? current.custom_request,
      };
      return { ...prev, features };
    });
  };

  const handleTopicsChange = (value: string) => {
    const topics = value
      .split(/[,;]/)
      .map((t) => t.trim())
      .filter(Boolean);
    update({ topics });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setSaving(true);
    try {
      if (id) {
        await updateNewsletter(id, form);
      } else {
        await createNewsletter(form);
      }
      navigate('/dashboard');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleSendSample = async () => {
    if (!id) return;
    setError(null);
    setSendingSample(true);
    try {
      await sendNewsletterSample(id);
      setError(null);
      alert('Sample sent to ' + (form.delivery_email || 'your delivery email') + '.');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Send sample failed');
    } finally {
      setSendingSample(false);
    }
  };

  if (loading) return <p>Loading…</p>;

  return (
    <div className="edit-newsletter">
      <h1>{id ? 'Edit newsletter' : 'New newsletter'}</h1>
      {error && <p className="error">{error}</p>}
      <form onSubmit={handleSubmit} className="newsletter-form">
        <label>
          Title
          <input
            type="text"
            value={form.title ?? ''}
            onChange={(e) => update({ title: e.target.value })}
            placeholder="My daily digest"
          />
        </label>
        <label>
          Daily send time (UTC)
          <input
            type="time"
            value={form.send_time_utc ?? '09:00'}
            onChange={(e) => update({ send_time_utc: e.target.value })}
          />
        </label>
        <label>
          Destination email address
          <input
            type="email"
            value={form.delivery_email ?? ''}
            onChange={(e) => update({ delivery_email: e.target.value })}
            required
            placeholder="you@example.com"
          />
        </label>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={form.is_active ?? true}
            onChange={(e) => update({ is_active: e.target.checked })}
          />
          Active (newsletter will be sent)
        </label>

        <div className="features">
          <p className="features-intro">
            Choose which sections to include and add optional custom instructions for OpenClaw.
          </p>
          {FEATURE_KEYS.map((key) => {
            const enabled = form.features?.[key]?.enabled ?? false;
            return (
              <div key={key} className="feature-row">
                <label className="toggle-label">
                  <input
                    type="checkbox"
                    className="toggle-input"
                    checked={enabled}
                    onChange={(e) => updateFeature(key, e.target.checked)}
                  />
                  <span className="toggle-switch" aria-hidden />
                  <span className="feature-name">{FEATURE_LABELS[key]}</span>
                </label>
                <div className="feature-request-block">
                  <p className="feature-description">
                    {FEATURE_DESCRIPTIONS[key]}
                  </p>
                  <textarea
                    placeholder={FEATURE_PLACEHOLDERS[key]}
                    value={form.features?.[key]?.custom_request ?? ''}
                    onChange={(e) => updateFeature(key, undefined, e.target.value)}
                    className="feature-request"
                    rows={3}
                    disabled={!enabled}
                  />
                </div>
              </div>
            );
          })}
        </div>

        <div className="form-actions">
          <button type="submit" disabled={saving}>
            {saving ? 'Saving…' : id ? 'Update' : 'Create'}
          </button>
          {id && (
            <button
              type="button"
              className="send-sample"
              onClick={handleSendSample}
              disabled={saving || sendingSample}
            >
              {sendingSample ? 'Sending…' : 'Send a sample now'}
            </button>
          )}
          <button type="button" onClick={() => navigate('/dashboard')}>
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
