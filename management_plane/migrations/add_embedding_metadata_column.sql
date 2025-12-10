-- Add embedding_metadata column to agent_policies table
-- This column stores metadata about ChromaDB synchronization for rule embeddings

ALTER TABLE agent_policies
ADD COLUMN embedding_metadata JSONB DEFAULT NULL;

COMMENT ON COLUMN agent_policies.embedding_metadata IS 'Metadata about rule embedding synchronization with ChromaDB (rule_id, chroma_synced_at, etc.)';
