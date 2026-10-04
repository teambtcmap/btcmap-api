-- Track the Gitea ticket workflow of a revoked submission on the submission
-- itself.
--
-- `revocation_action` records the decision (`close` for a ticket that was still
-- open when the submission was revoked, `reopen` for one that had already been
-- processed) so a retry re-applies the same action instead of inferring it again
-- from a ticket state the previous attempt may have changed itself.
-- `revocation_processed_at` is set once every Gitea call for that submission has
-- succeeded, which is what lets the sync stop looking at finished rows.
ALTER TABLE place_submission ADD COLUMN revocation_action TEXT;
ALTER TABLE place_submission ADD COLUMN revocation_processed_at TEXT;
