-- Composite index for common incident queries:
--   - active incidents: WHERE workspace_id = ? AND status_page_id = ? AND status <> 'resolved'
--   - resolved incidents: WHERE workspace_id = ? AND status_page_id = ? AND status = 'resolved'
--   - history: WHERE workspace_id = ? AND status_page_id = ? AND started_at >= ?
CREATE INDEX IF NOT EXISTS idx_incidents_workspace_page_status_started
  ON incidents(workspace_id, status_page_id, status, started_at DESC);
