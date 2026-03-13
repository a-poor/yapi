-- set some pragma stuff
pragma foreign_keys = 1;
pragma journal_mode = wal;

begin;

-- create the migrations table
create table _migrations (
    version integer not null
);

-- create the other tables
create table workspaces (
    id integer primary key autoincrement,
    name text unique not null,
    description text not null default '',
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec'))
);

create table environments (
    id integer primary key autoincrement,
    workspace_id integer not null references workspaces(id),
    name text not null,
    description text not null default '',
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec')),
    unique(workspace_id, name)
);

create table environment_vars (
    id integer primary key autoincrement,
    env_id integer not null references environments(id),
    name text not null,
    description text not null default '',
    value text not null default '',
    is_secret integer not null default 0,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec')),
    unique(env_id, name)
);

create table collections (
    id integer primary key autoincrement,
    workspace_id integer not null references workspaces(id),
    name text not null,
    description text not null default '',
    default_env integer references environments(id),
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec')),
    unique(workspace_id, name)
);

create table collection_vars (
    id integer primary key autoincrement,
    coll_id integer not null references collections(id),
    name text not null,
    description text not null default '',
    value text not null default '',
    is_secret integer not null default 0,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec')),
    unique(coll_id, name)
);

create table requests (
    id integer primary key autoincrement,
    coll_id integer not null references collections(id),
    name text not null,
    method text not null default 'GET',
    url text not null,
    body text,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec')),
    unique(coll_id, name)
);

create table request_headers (
    id integer primary key autoincrement,
    req_id integer not null references requests(id),
    hkey text not null,
    hval text not null,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec'))
);

create table request_query_params (
    id integer primary key autoincrement,
    req_id integer not null references requests(id),
    qkey text not null,
    qval text not null,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec'))
);

create table history (
    id integer primary key autoincrement,
    req_id integer references requests(id),
    method text not null,
    resolved_url text not null, -- req url after substitution
    resolved_req_headers text not null default '[]', -- json: [{"key":"","value":""}]
    resolved_req_body text, -- body, if any
    success integer not null, -- was response returned
    res_status integer, -- status code
    res_body text, -- body, if any
    res_headers text not null default '[]', -- json (like for req)
    res_duration real,
    created_at datetime not null default (datetime('subsec')),
    updated_at datetime not null default (datetime('subsec'))
);

-- add a default workspace?
insert into workspaces (name) values ('default');

-- log the first migration
insert into _migrations (version) values (0);

-- done
commit;
