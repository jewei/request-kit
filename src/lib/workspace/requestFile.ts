/**
 * Pure mappings between a tab's editable `RequestDraft` and the persisted
 * `RequestFile` document. Kept side-effect-free so both directions are unit
 * tested without stores or IPC.
 */
import type { RequestDraft } from '../prepare/types';
import type { RequestFile } from '../../types/workspace';

/** Build the on-disk document for a request. `auth`/`variables` use M2a defaults. */
export function draftToRequestFile(id: string, name: string, draft: RequestDraft): RequestFile {
  return {
    version: 1,
    id,
    name,
    method: draft.method,
    url: draft.url,
    headers: draft.headers,
    body: draft.body,
    auth: { type: 'inherit' },
    variables: [],
    settings: draft.settings,
  };
}

/** Extract the editable draft from a stored request document. */
export function requestFileToDraft(file: RequestFile): RequestDraft {
  return {
    method: file.method,
    url: file.url,
    headers: file.headers,
    body: file.body,
    settings: file.settings,
  };
}
