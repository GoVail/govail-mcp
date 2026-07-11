create schema if not exists grc;

create table if not exists grc.evidence_bundles (
    id uuid primary key,
    project_id text not null,
    source text not null,
    asset_type text,
    asset_name text,
    bundle jsonb not null,
    validation jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now()
);

create index if not exists idx_evidence_bundles_project_created
    on grc.evidence_bundles (project_id, created_at desc);

create table if not exists grc.findings (
    id uuid primary key,
    bundle_id uuid not null references grc.evidence_bundles(id) on delete cascade,
    project_id text not null,
    severity text not null,
    category text not null,
    title text not null,
    finding jsonb not null,
    created_at timestamptz not null default now()
);

create index if not exists idx_findings_project_severity
    on grc.findings (project_id, severity, created_at desc);

create table if not exists grc.control_mappings (
    id uuid primary key,
    finding_id uuid not null references grc.findings(id) on delete cascade,
    framework text not null,
    control_id text not null,
    rationale text not null,
    confidence numeric not null default 0.5,
    created_at timestamptz not null default now()
);

create index if not exists idx_control_mappings_framework
    on grc.control_mappings (framework, control_id);
