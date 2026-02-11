import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import type { Session } from '@supabase/supabase-js';
import { supabase } from '../supabaseClient';
import { getApprovalStatus } from '../lib/api';
import { Link, useNavigate } from 'react-router-dom';

type AuthGateProps = {
  children: ReactNode;
};

export function AuthGate({ children }: AuthGateProps) {
  const navigate = useNavigate();
  const [session, setSession] = useState<Session | null>(null);
  const [approved, setApproved] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [isSignUp, setIsSignUp] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session);
      setLoading(false);
    });
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session);
      setApproved(null);
    });
    return () => subscription.unsubscribe();
  }, []);

  useEffect(() => {
    if (!session) {
      setApproved(null);
      return;
    }
    getApprovalStatus()
      .then(({ approved: a }) => setApproved(a))
      .catch(() => setApproved(false));
  }, [session]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setAuthError(null);
    try {
      if (isSignUp) {
        const { data, error } = await supabase.auth.signUp({ email, password });
        if (error) throw error;
        if (data.session) navigate('/newsletters/new', { replace: true });
      } else {
        const { data, error } = await supabase.auth.signInWithPassword({ email, password });
        if (error) throw error;
        if (data.session) navigate('/newsletters/new', { replace: true });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Auth failed';
      if (isSignUp && message === 'Database error saving new user') {
        setAuthError('An account with this email may already exist. Try signing in instead.');
      } else {
        setAuthError(message);
      }
    }
  };

  const handleSignOut = () => {
    supabase.auth.signOut();
  };

  if (loading) {
    return (
      <div className="auth-loading">
        <p>Loading…</p>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="auth-page">
        <h1>Speedo.email</h1>
        <p>A personalized executive summary <br /> of all the daily info you need to run your busines. <br /> in an email newsletter.</p>
        <form onSubmit={handleSubmit} className="auth-form">
          <input
            type="email"
            placeholder="Email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
          <input
            type="password"
            placeholder="Password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          {authError && <p className="auth-error">{authError}</p>}
          <button type="submit">{isSignUp ? 'Sign up' : 'Sign in'}</button>
          <button type="button" onClick={() => setIsSignUp(!isSignUp)} className="auth-toggle">
            {isSignUp ? 'Already have an account? Sign in' : 'Need an account? Sign up'}
          </button>
        </form>
      </div>
    );
  }

  if (approved === null) {
    return (
      <div className="auth-loading">
        <p>Loading…</p>
      </div>
    );
  }

  if (!approved) {
    return (
      <>
        <header className="app-header">
          <span className="user-email">{session.user?.email}</span>
          <button type="button" onClick={handleSignOut} className="sign-out">
            Sign out
          </button>
        </header>
        <main className="app-main">
          <div className="auth-page">
            <h1>Thanks for registering</h1>
            <p>
              In order to keep costs down for now, only manually approved users get access. Standby—we&apos;ll enable your account soon.
            </p>
          </div>
        </main>
      </>
    );
  }

  return (
    <>
      <header className="app-header">
        <nav>
          <Link to="/dashboard">Dashboard</Link>
          <Link to="/newsletters/new">New newsletter</Link>
        </nav>
        <span className="user-email">{session.user?.email}</span>
        <button type="button" onClick={handleSignOut} className="sign-out">
          Sign out
        </button>
      </header>
      <main className="app-main">{children}</main>
    </>
  );
}
