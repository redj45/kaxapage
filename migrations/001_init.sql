-- migrations/001_init.sql

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS citext;

DO $$ BEGIN
  CREATE TYPE service_status AS ENUM ('operational','degraded','partial_outage','major_outage','maintenance');
EXCEPTION WHEN duplicate_object THEN null; END $$;

DO $$ BEGIN
  CREATE TYPE incident_status AS ENUM ('investigating','identified','monitoring','resolved');
EXCEPTION WHEN duplicate_object THEN null; END $$;

DO $$ BEGIN
  CREATE TYPE incident_impact AS ENUM ('none','minor','major','critical');
EXCEPTION WHEN duplicate_object THEN null; END $$;

CREATE TABLE IF NOT EXISTS workspaces (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS status_pages (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id uuid NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  slug text NOT NULL,
  title text NOT NULL,
  published boolean NOT NULL DEFAULT true,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (workspace_id, slug)
);

CREATE TABLE IF NOT EXISTS services (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id uuid NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  name text NOT NULL,
  description text,
  position int NOT NULL DEFAULT 0,
  status service_status NOT NULL DEFAULT 'operational',
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_services_workspace_position
  ON services(workspace_id, position);

CREATE TABLE IF NOT EXISTS incidents (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id uuid NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  status_page_id uuid NOT NULL REFERENCES status_pages(id) ON DELETE CASCADE,
  title text NOT NULL,
  status incident_status NOT NULL DEFAULT 'investigating',
  impact incident_impact NOT NULL DEFAULT 'minor',
  started_at timestamptz NOT NULL DEFAULT now(),
  resolved_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_incidents_page_started
  ON incidents(status_page_id, started_at DESC);

CREATE TABLE IF NOT EXISTS incident_updates (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  incident_id uuid NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
  message text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_incident_updates_incident_created
  ON incident_updates(incident_id, created_at);

CREATE TABLE IF NOT EXISTS subscribers (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  status_page_id uuid NOT NULL REFERENCES status_pages(id) ON DELETE CASCADE,
  email citext NOT NULL,
  verified boolean NOT NULL DEFAULT false,
  verify_token text,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (status_page_id, email)
);

CREATE INDEX IF NOT EXISTS idx_subscribers_page_verified
  ON subscribers(status_page_id, verified);
