import { supabase } from '../supabaseClient';
import type { NewsletterConfig } from '../types';

const API_BASE = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '');

async function getAuthHeaders(): Promise<HeadersInit> {
  const {
    data: { session },
  } = await supabase.auth.getSession();
  const headers: HeadersInit = { 'Content-Type': 'application/json' };
  if (session?.access_token) {
    (headers as Record<string, string>)['Authorization'] = `Bearer ${session.access_token}`;
  }
  return headers;
}

export async function apiGet<T>(path: string): Promise<T> {
  const headers = await getAuthHeaders();
  const res = await fetch(`${API_BASE}${path}`, { headers });
  if (!res.ok) throw new Error(await res.text().catch(() => res.statusText));
  return res.json();
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  const headers = await getAuthHeaders();
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(await res.text().catch(() => res.statusText));
  return res.json();
}

export async function apiPut<T>(path: string, body: unknown): Promise<T> {
  const headers = await getAuthHeaders();
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'PUT',
    headers,
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(await res.text().catch(() => res.statusText));
  return res.json();
}

export async function apiDelete(path: string): Promise<void> {
  const headers = await getAuthHeaders();
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'DELETE',
    headers,
  });
  if (!res.ok) throw new Error(await res.text().catch(() => res.statusText));
}

export async function listNewsletters(): Promise<NewsletterConfig[]> {
  return apiGet<NewsletterConfig[]>('/api/me/newsletters');
}

export async function getNewsletter(id: string): Promise<NewsletterConfig> {
  return apiGet<NewsletterConfig>(`/api/me/newsletters/${id}`);
}

export type CreateNewsletterBody = {
  title?: string;
  topics?: string[];
  tone?: string;
  length?: string;
  send_time_utc?: string;
  timezone?: string;
  delivery_email?: string;
  is_active?: boolean;
  features?: Record<string, { enabled: boolean; custom_request: string }>;
};

export async function createNewsletter(body: CreateNewsletterBody): Promise<NewsletterConfig> {
  return apiPost<NewsletterConfig>('/api/me/newsletters', body);
}

export async function updateNewsletter(id: string, body: CreateNewsletterBody): Promise<NewsletterConfig> {
  return apiPut<NewsletterConfig>(`/api/me/newsletters/${id}`, body);
}

export async function deleteNewsletter(id: string): Promise<void> {
  return apiDelete(`/api/me/newsletters/${id}`);
}

export async function sendNewsletterSample(
  id: string,
  body?: CreateNewsletterBody,
): Promise<{ sent: boolean }> {
  const headers = await getAuthHeaders();
  const res = await fetch(`${API_BASE}/api/me/newsletters/${id}/send-sample`, {
    method: 'POST',
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(await res.text().catch(() => res.statusText));
  return res.json();
}

export type ApprovalStatus = { approved: boolean };

export async function getApprovalStatus(): Promise<ApprovalStatus> {
  return apiGet<ApprovalStatus>('/api/me/approval-status');
}
