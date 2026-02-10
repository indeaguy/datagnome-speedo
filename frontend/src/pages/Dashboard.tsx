import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listNewsletters, deleteNewsletter } from '../lib/api';
import type { NewsletterConfig } from '../types';

export function Dashboard() {
  const [configs, setConfigs] = useState<NewsletterConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    listNewsletters()
      .then(setConfigs)
      .catch((e) => setError(e instanceof Error ? e.message : 'Failed to load'))
      .finally(() => setLoading(false));
  }, []);

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.preventDefault();
    if (!confirm('Delete this newsletter config?')) return;
    try {
      await deleteNewsletter(id);
      setConfigs((prev) => prev.filter((c) => c.id !== id));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Delete failed');
    }
  };

  if (loading) return <p>Loading…</p>;
  if (error) return <p className="error">{error}</p>;

  return (
    <div className="dashboard">
      <h1>Your newsletters</h1>
      {configs.length === 0 ? (
        <p>No newsletters yet. <Link to="/newsletters/new">Create one</Link>.</p>
      ) : (
        <ul className="newsletter-list">
          {configs.map((c) => (
            <li key={c.id} className="newsletter-item">
              <Link to={`/newsletters/${c.id}/edit`}>
                <strong>{c.title || 'Untitled'}</strong>
                <span> {c.delivery_email}</span>
                <span> {c.is_active ? 'Active' : 'Paused'}</span>
              </Link>
              <button
                type="button"
                onClick={(e) => handleDelete(c.id, e)}
                className="delete-btn"
                aria-label="Delete"
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
