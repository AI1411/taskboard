export type FieldDraft = {
  value: string;
  base: string;
  dirty: boolean;
  conflict: boolean;
};

export function freshDraft(server: string): FieldDraft {
  return { value: server, base: server, dirty: false, conflict: false };
}

export function followServer(draft: FieldDraft, server: string): FieldDraft {
  if (!draft.dirty || server === draft.value) {
    if (draft.value === server && draft.base === server && !draft.conflict) return draft;
    return freshDraft(server);
  }
  if (server !== draft.base) {
    if (draft.conflict) return draft;
    return { ...draft, conflict: true };
  }
  if (draft.conflict) return { ...draft, conflict: false };
  return draft;
}

export function editDraft(draft: FieldDraft, value: string): FieldDraft {
  if (value === draft.value) return draft;
  const dirty = value !== draft.base;
  return {
    value,
    base: draft.base,
    dirty,
    conflict: dirty ? draft.conflict : false,
  };
}

export function shouldCommit(draft: FieldDraft): boolean {
  return draft.dirty && !draft.conflict && draft.value !== draft.base;
}

export function keepMine(draft: FieldDraft, server: string): FieldDraft {
  return {
    value: draft.value,
    base: server,
    dirty: draft.value !== server,
    conflict: false,
  };
}
