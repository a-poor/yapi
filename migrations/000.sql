begin;
-- Create the migrations table
create table _migrations (
    version int not null
);

-- Set some pragma stuff
pragma foreign_keys=1;
pragma journal_mode=WAL;

-- Create the other tables
create table workspaces (
    id int primary key autoincrement,
    name text unique not null,
    description text not null default '',
    default_env int references environments(id),
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec')
);

create table environments (
    id int primary key autoincrement,
    workspace_id int not null references workspaces(id),
    name text not null,
    description text not null default '',
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec'),
    unique(workspace_id, name)
);

create table environment_vars (
    id int primary key autoincrement,
    env_id int not null references environments(id),
    name text not null,
    description text not null default '',
    value text not null default '',
    is_secret int not null default 0,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec'),
    unique(env_id, name)
);

create table collections (
    id int primary key autoincrement,
    workspace_id int not null references workspaces(id),
    name text not null,
    description text not null default '',
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec'),
    unique(workspace_id, name)
);

create table collection_vars (
    id int primary key autoincrement,
    coll_id int not null references collections(id),
    name text not null,
    description text not null default '',
    value text not null default '',
    is_secret int not null default 0,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec'),
    unique(env_id, name)
);

create table requests (
    id int primary key autoincrement,
    coll_id int not null references collections(id),
    name text not null,
    method text not null default 'GET',
    url text not null,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec'),
    unique(coll_id, name)
);

create table request_headers (
    id int primary key autoincrement,
    req_id int not null references requests(id),
    hkey text not null,
    hval text not null,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec')
);

create table request_query_params (
    id int primary key autoincrement,
    req_id int not null references requests(id),
    qkey text not null,
    qval text not null,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec')
);

create table history (
    id int primary key autoincrement,
    req_id int references collections(id),
    method text not null,
    resolved_url text not null, -- req url after substitution
    resolved_req_headers text not null default '[]', -- JSON: [{"key":"","value":""}]
    resolved_req_body text, -- body, if any
    success int not null, -- was response returned
    res_status int, -- status code
    res_body text, -- body, if any
    res_headers text not null, -- JSON (like for req)
    res_duration real,
    created_at datetime not null default datetime('subsec'),
    updated_at datetime not null default datetime('subsec')
);

-- Add a default workspace
insert into workspaces (name) values ('default');

-- Log the first migration
insert into _migrations (0);

-- done
commit;
