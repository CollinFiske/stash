import { load, type Store } from "@tauri-apps/plugin-store";
import type { SavedGroup, ProjectTemplate } from "./types";

const STORE_FILE = "workspaces.json";
const GROUPS_KEY = "saved_groups";
const GROUP_ORDER_KEY = "group_order";
const TEMPLATES_KEY = "project_templates";
const TEMPLATE_ORDER_KEY = "template_order";

let _store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!_store) {
    _store = await load(STORE_FILE, { defaults: {}, autoSave: false });
  }
  return _store;
}

export async function loadGroups(): Promise<Record<string, SavedGroup>> {
  const store = await getStore();
  const groups = await store.get<Record<string, SavedGroup>>(GROUPS_KEY);
  return groups ?? {};
}

export async function saveGroup(group: SavedGroup): Promise<void> {
  const store = await getStore();
  const existing = await loadGroups();
  existing[group.name] = group;
  await store.set(GROUPS_KEY, existing);
  await store.save();
}

export async function deleteGroup(name: string): Promise<void> {
  const store = await getStore();
  const existing = await loadGroups();
  delete existing[name];
  await store.set(GROUPS_KEY, existing);
  await store.save();
}

// ── Project Templates ─────────────────────────────────────────────────────────

export async function loadTemplates(): Promise<Record<string, ProjectTemplate>> {
  const store = await getStore();
  const templates = await store.get<Record<string, ProjectTemplate>>(TEMPLATES_KEY);
  return templates ?? {};
}

export async function saveTemplate(template: ProjectTemplate): Promise<void> {
  const store = await getStore();
  const existing = await loadTemplates();
  existing[template.name] = template;
  await store.set(TEMPLATES_KEY, existing);
  await store.save();
}

export async function deleteTemplate(name: string): Promise<void> {
  const store = await getStore();
  const existing = await loadTemplates();
  delete existing[name];
  await store.set(TEMPLATES_KEY, existing);
  await store.save();
}

// ── Update (rename + modify) ──────────────────────────────────────────────────

export async function updateGroup(originalName: string, updated: SavedGroup): Promise<void> {
  const store = await getStore();
  const existing = await loadGroups();
  if (originalName !== updated.name) delete existing[originalName];
  existing[updated.name] = updated;
  await store.set(GROUPS_KEY, existing);
  await store.save();
}

export async function updateTemplate(originalName: string, updated: ProjectTemplate): Promise<void> {
  const store = await getStore();
  const existing = await loadTemplates();
  if (originalName !== updated.name) delete existing[originalName];
  existing[updated.name] = updated;
  await store.set(TEMPLATES_KEY, existing);
  await store.save();
}

// ── Ordering ──────────────────────────────────────────────────────────────────

export async function loadGroupOrder(): Promise<string[]> {
  const store = await getStore();
  const order = await store.get<string[]>(GROUP_ORDER_KEY);
  return order ?? [];
}

export async function saveGroupOrder(order: string[]): Promise<void> {
  const store = await getStore();
  await store.set(GROUP_ORDER_KEY, order);
  await store.save();
}

export async function loadTemplateOrder(): Promise<string[]> {
  const store = await getStore();
  const order = await store.get<string[]>(TEMPLATE_ORDER_KEY);
  return order ?? [];
}

export async function saveTemplateOrder(order: string[]): Promise<void> {
  const store = await getStore();
  await store.set(TEMPLATE_ORDER_KEY, order);
  await store.save();
}
