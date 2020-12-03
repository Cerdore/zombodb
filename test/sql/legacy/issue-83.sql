CREATE TABLE public.documents (
  pk_documents SERIAL8,
  doc_stuff    TEXT,
  CONSTRAINT idx_documents PRIMARY KEY (pk_documents)
);
CREATE TABLE public.docs_usage (
  pk_docs_usage      SERIAL8,
  fk_documents       BIGINT,
  fk_library_profile BIGINT,
  place_used         TEXT,
  CONSTRAINT idx_docs_usage PRIMARY KEY (pk_docs_usage)
);
CREATE TABLE public.library_profile (
  pk_library_profile SERIAL8,
  library_name       TEXT,
  CONSTRAINT idx_library_profile PRIMARY KEY (pk_library_profile)
);

CREATE INDEX es_documents ON documents USING zombodb ((documents.*));
CREATE INDEX es_docs_usage ON docs_usage USING zombodb ((docs_usage.*));
CREATE INDEX es_library_profile ON library_profile USING zombodb ((library_profile.*));

CREATE OR REPLACE VIEW documents_master_view AS
  SELECT
    documents.*,
    (SELECT json_agg(row_to_json(du.*)) AS json_agg
     FROM (SELECT
             docs_usage.*,
             (SELECT library_profile.library_name
              FROM library_profile
              WHERE library_profile.pk_library_profile = docs_usage.fk_library_profile) AS library_name
           FROM docs_usage
           WHERE documents.pk_documents = docs_usage.fk_documents) du) AS usage_data,

    documents AS zdb
  FROM public.documents;
COMMENT ON VIEW documents_master_view IS $${
    "index": "public.es_documents",
    "options": [
                "docs_usage_data:(pk_documents=<public.docs_usage.es_docs_usage>fk_documents)",
                "fk_library_profile=<public.library_profile.es_library_profile>pk_library_profile"
    ]
}$$;

INSERT INTO documents (doc_stuff)
VALUES ('Every good boy does fine.'), ('Sally sells sea shells down by the seashore.'),
  ('The quick brown fox jumps over the lazy dog.');
INSERT INTO library_profile (library_name) VALUES ('GSO Public Library'), ('Library of Congress'), ('The interwebs.');
INSERT INTO docs_usage (fk_documents, fk_library_profile, place_used)
VALUES (1, 1, 'somewhere'), (2, 2, 'anywhere'), (3, 3, 'everywhere'), (3, 1, 'somewhere');

SELECT count(*) FROM documents_master_view WHERE public.documents_master_view.zdb ==> 'somewhere';
SELECT count(*) FROM documents_master_view WHERE public.documents_master_view.zdb ==> 'GSO';

DROP TABLE documents CASCADE;
DROP TABLE docs_usage CASCADE;
DROP TABLE library_profile CASCADE;
