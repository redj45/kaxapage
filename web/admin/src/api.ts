const LS = {
  pageId: "kp_page_id",
  pageSlug: "kp_page_slug",
};

export function getPageId() { return localStorage.getItem(LS.pageId) || ""; }
export function setPageId(v: string) { localStorage.setItem(LS.pageId, v); }

export function getPageSlug() { return localStorage.getItem(LS.pageSlug) || ""; }
export function setPageSlug(v: string) { localStorage.setItem(LS.pageSlug, v); }

export type ApiOk<T> = { ok: true; data: T };
export type ApiErr = { ok: false; error: { code: string; message: string } };
export type ApiResp<T> = ApiOk<T> | ApiErr;

export type ServiceStatus = "operational" | "degraded" | "partial_outage" | "major_outage" | "maintenance";
export type IncidentStatus = "investigating" | "identified" | "monitoring" | "resolved";
export type IncidentImpact = "none" | "minor" | "major" | "critical";

export type PageItem = { id: string; slug: string; title: string; published: boolean };
export type ServiceItem = { id: string; name: string; description?: string | null; position: number; status: ServiceStatus; updated_at: string };

const API_BASE = "/api/v1";

// Cookies are sent automatically for same-origin requests; no Authorization header needed.
async function req<T>(path: string, init?: RequestInit): Promise<{ status: number; body: ApiResp<T> | null }> {
  const headers = new Headers(init?.headers || {});
  headers.set("Content-Type", "application/json");

  const res = await fetch(API_BASE + path, { ...init, headers });
  if (res.status === 204) return { status: 204, body: null };

  const body = (await res.json().catch(() => null)) as ApiResp<T> | null;
  return { status: res.status, body };
}

export const api = {
  login: (token: string) => req<{}>("/admin/login", {
    method: "POST",
    body: JSON.stringify({ token }),
  }),
  logout: () => req<{}>("/admin/logout", { method: "POST" }),

  pages: () => req<PageItem[]>("/admin/pages"),
  bootstrap: (payload: any) => req<{ workspace_id: string; status_page_id: string; page_slug: string }>("/bootstrap", {
    method: "POST",
    body: JSON.stringify(payload),
  }),
  services: () => req<ServiceItem[]>("/admin/services"),
  createService: (payload: any) => req<ServiceItem>("/admin/services", { method: "POST", body: JSON.stringify(payload) }),
  patchService: (id: string, payload: any) => req<ServiceItem>(`/admin/services/${id}`, { method: "PATCH", body: JSON.stringify(payload) }),
  deleteService: (id: string) => req<never>(`/admin/services/${id}`, { method: "DELETE" }),

  incidents: (pageId: string, limit = 30) =>
    req<{ items: any[]; next_cursor: any }>(`/admin/incidents?status_page_id=${encodeURIComponent(pageId)}&limit=${limit}`),
  createIncident: (payload: any) => req<any>("/admin/incidents", { method: "POST", body: JSON.stringify(payload) }),
  getIncident: (id: string) => req<any>(`/admin/incidents/${id}`),
  addUpdate: (id: string, payload: any) => req<any>(`/admin/incidents/${id}/updates`, { method: "POST", body: JSON.stringify(payload) }),
  resolveIncident: (id: string, payload: any) => req<any>(`/admin/incidents/${id}/resolve`, { method: "POST", body: JSON.stringify(payload) }),

  publicPage: (slug: string) => req<any>(`/public/pages/${slug}`),
};
