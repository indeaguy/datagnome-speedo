import { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  getNewsletter,
  createNewsletter,
  updateNewsletter,
  type CreateNewsletterBody,
} from '../lib/api';
import { supabase } from '../supabaseClient';
import { DEFAULT_FEATURES } from '../types';

const TONES = ['neutral', 'playful', 'serious', 'professional'];
const LENGTHS = ['short', 'medium', 'long'];
const FEATURE_KEYS = ['competitor_analysis', 'market_segment_summary', 'identify_risks'] as const;
const FEATURE_LABELS: Record<string, string> = {
  competitor_analysis: 'Competitor analysis',
  market_segment_summary: 'Market segment summary',
  identify_risks: 'Identify risks',
};

export function EditNewsletter() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [loading, setLoading] = useState(!!id);
  const [saving, setSaving] = useState(false);
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
          Topics (comma-separated)
          <input
            type="text"
            value={(form.topics ?? []).join(', ')}
            onChange={(e) => handleTopicsChange(e.target.value)}
            placeholder="AI, markets, tech"
          />
        </label>
        <label>
          Tone
          <select
            value={form.tone ?? 'neutral'}
            onChange={(e) => update({ tone: e.target.value })}
          >
            {TONES.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </label>
        <label>
          Length
          <select
            value={form.length ?? 'medium'}
            onChange={(e) => update({ length: e.target.value })}
          >
            {LENGTHS.map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>
        </label>
        <label>
          Send time (UTC)
          <input
            type="time"
            value={form.send_time_utc ?? '09:00'}
            onChange={(e) => update({ send_time_utc: e.target.value })}
          />
        </label>
        <label>
          Timezone
          <input
            type="text"
            value={form.timezone ?? 'UTC'}
            onChange={(e) => update({ timezone: e.target.value })}
            placeholder="America/New_York"
          />
        </label>
        <label>
          Delivery email
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

        <fieldset className="features">
          <legend>Sections (enable and add custom instructions for OpenClaw)</legend>
          {FEATURE_KEYS.map((key) => (
            <div key={key} className="feature-row">
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={form.features?.[key]?.enabled ?? false}
                  onChange={(e) => updateFeature(key, e.target.checked)}
                />
                {FEATURE_LABELS[key]}
              </label>
              <input
                type="text"
                placeholder="Custom request for this section (optional)"
                value={form.features?.[key]?.custom_request ?? ''}
                onChange={(e) => updateFeature(key, undefined, e.target.value)}
                className="feature-request"
              />
            </div>
          ))}
        </fieldset>

        <div className="form-actions">
          <button type="submit" disabled={saving}>
            {saving ? 'Saving…' : id ? 'Update' : 'Create'}
          </button>
          <button type="button" onClick={() => navigate('/dashboard')}>
            Cancel
          </button>
        </div>
      </form>
    </div>
  );
}
