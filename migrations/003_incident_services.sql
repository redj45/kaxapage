-- Track which services are affected by each incident.
CREATE TABLE IF NOT EXISTS incident_services (
  incident_id uuid NOT NULL REFERENCES incidents(id) ON DELETE CASCADE,
  service_id  uuid NOT NULL REFERENCES services(id)  ON DELETE CASCADE,
  PRIMARY KEY (incident_id, service_id)
);

-- Subscribers table has no implementation — remove dead schema.
DROP TABLE IF EXISTS subscribers;
