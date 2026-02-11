-- Users in this table are allowed to use the app. Toggle access by inserting (approve) or deleting (revoke).
CREATE TABLE approved_users (
  user_id uuid PRIMARY KEY
);

-- Only backend (service role) can read; anon/authenticated see no rows.
ALTER TABLE approved_users ENABLE ROW LEVEL SECURITY;

CREATE POLICY "service_role_only" ON approved_users
  FOR ALL
  USING (false)
  WITH CHECK (false);
