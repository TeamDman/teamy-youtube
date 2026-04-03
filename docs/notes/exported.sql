-- Drop table

-- DROP TABLE youtube.channel_embeddings_bge_m3;

CREATE TABLE youtube.channel_embeddings_bge_m3 (
	channel_id text NOT NULL,
	embedded_on timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
	embedding public.vector NULL,
	CONSTRAINT channel_embeddings_bge_m3_pkey PRIMARY KEY (channel_id)
);

-- Drop table

-- DROP TABLE youtube.missing_videos;

CREATE TABLE youtube.missing_videos (
	video_id text NOT NULL,
	fetched_on timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
	CONSTRAINT missing_videos_pkey PRIMARY KEY (video_id)
);

-- Drop table

-- DROP TABLE youtube.posts;

CREATE TABLE youtube.posts (
	"time" timestamp NOT NULL,
	post_title varchar(8192) NOT NULL,
	post_url text NOT NULL,
	channel_url text NOT NULL,
	channel_name varchar(128) NOT NULL,
	CONSTRAINT posts_pkey PRIMARY KEY ("time")
);

-- Drop table

-- DROP TABLE youtube.search_history;

CREATE TABLE youtube.search_history (
	"time" timestamp NOT NULL,
	query varchar(256) NOT NULL,
	CONSTRAINT search_history_pkey PRIMARY KEY ("time")
);

-- Drop table

-- DROP TABLE youtube.video_categories;

CREATE TABLE youtube.video_categories (
	id text NOT NULL,
	title text NOT NULL,
	assignable bool NOT NULL,
	channel_id text NOT NULL,
	CONSTRAINT video_categories_pkey PRIMARY KEY (id)
);

-- Drop table

-- DROP TABLE youtube.video_embeddings_bge_m3;

CREATE TABLE youtube.video_embeddings_bge_m3 (
	video_etag text NOT NULL,
	embedded_on timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
	embedding public.vector NULL,
	CONSTRAINT video_embeddings_bge_m3_pkey PRIMARY KEY (video_etag),
	CONSTRAINT video_embeddings_bge_m3_video_etag_fkey FOREIGN KEY (video_etag) REFERENCES youtube.videos(etag) ON DELETE CASCADE
);

-- Drop table

-- DROP TABLE youtube.video_thumbnails;

CREATE TABLE youtube.video_thumbnails (
	id serial4 NOT NULL,
	video_etag text NULL,
	size_description text NOT NULL,
	height int4 NULL,
	width int4 NULL,
	url text NOT NULL,
	CONSTRAINT video_thumbnails_pkey PRIMARY KEY (id),
	CONSTRAINT video_thumbnails_video_etag_fkey FOREIGN KEY (video_etag) REFERENCES youtube.videos(etag) ON DELETE CASCADE
);

-- Drop table

-- DROP TABLE youtube.video_topics;

CREATE TABLE youtube.video_topics (
	id serial4 NOT NULL,
	video_etag text NULL,
	topic_url text NOT NULL,
	CONSTRAINT video_topics_pkey PRIMARY KEY (id),
	CONSTRAINT video_topics_video_etag_fkey FOREIGN KEY (video_etag) REFERENCES youtube.videos(etag) ON DELETE CASCADE
);

-- Drop table

-- DROP TABLE youtube.videos;

CREATE TABLE youtube.videos (
	etag text NOT NULL,
	video_id text NOT NULL,
	fetched_on timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
	title text NOT NULL,
	description text NOT NULL,
	published_at timestamp NOT NULL,
	channel_id text NOT NULL,
	channel_title text NOT NULL,
	category_id text NOT NULL,
	duration interval NOT NULL,
	caption bool NOT NULL,
	definition text NOT NULL,
	dimension text NOT NULL,
	licensed_content bool NOT NULL,
	privacy_status text NOT NULL,
	tags _text NULL,
	view_count int8 NULL,
	like_count int8 NULL,
	comment_count int8 NULL,
	search_document tsvector NOT NULL,
	CONSTRAINT videos_pkey PRIMARY KEY (etag)
);
CREATE INDEX idx_videos_search_document_gin ON youtube.videos USING gin (search_document);

-- Table Triggers

create trigger trigger_update_search_doc before
insert
    or
update
    of title,
    description,
    channel_title on
    youtube.videos for each row execute function youtube.update_search_document();

-- Drop table

-- DROP TABLE youtube.watch_history;

CREATE TABLE youtube.watch_history (
	"time" timestamp NOT NULL,
	youtube_video_id varchar(16) NOT NULL,
	CONSTRAINT watch_history_pkey PRIMARY KEY ("time")
);

CREATE UNIQUE INDEX channel_embeddings_bge_m3_pkey ON youtube.channel_embeddings_bge_m3 USING btree (channel_id);

CREATE UNIQUE INDEX missing_videos_pkey ON youtube.missing_videos USING btree (video_id);

CREATE UNIQUE INDEX posts_pkey ON youtube.posts USING btree ("time");

CREATE UNIQUE INDEX search_history_pkey ON youtube.search_history USING btree ("time");

CREATE UNIQUE INDEX video_categories_pkey ON youtube.video_categories USING btree (id);

CREATE UNIQUE INDEX video_embeddings_bge_m3_pkey ON youtube.video_embeddings_bge_m3 USING btree (video_etag);

CREATE UNIQUE INDEX video_thumbnails_pkey ON youtube.video_thumbnails USING btree (id);

CREATE UNIQUE INDEX video_topics_pkey ON youtube.video_topics USING btree (id);

CREATE INDEX idx_videos_search_document_gin ON youtube.videos USING gin (search_document);

CREATE UNIQUE INDEX videos_pkey ON youtube.videos USING btree (etag);

CREATE UNIQUE INDEX watch_history_pkey ON youtube.watch_history USING btree ("time");

-- DROP FUNCTION youtube.update_search_document();

CREATE OR REPLACE FUNCTION youtube.update_search_document()
 RETURNS trigger
 LANGUAGE plpgsql
AS $function$
BEGIN
    NEW.search_document := to_tsvector('english',
        coalesce(NEW.title, '') || ' ' ||
        coalesce(NEW.description, '') || ' ' ||
        coalesce(NEW.channel_title, '')
    );
    RETURN NEW;
END;
$function$
;

-- DROP SEQUENCE youtube.video_thumbnails_id_seq;

CREATE SEQUENCE youtube.video_thumbnails_id_seq
	INCREMENT BY 1
	MINVALUE 1
	MAXVALUE 2147483647
	START 1
	CACHE 1
	NO CYCLE;

-- DROP SEQUENCE youtube.video_topics_id_seq;

CREATE SEQUENCE youtube.video_topics_id_seq
	INCREMENT BY 1
	MINVALUE 1
	MAXVALUE 2147483647
	START 1
	CACHE 1
	NO CYCLE;