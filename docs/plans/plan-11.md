# Phase 11: Marketplace — Complete Implementation

Searchable registry of contract templates with one-click deploy and community contributions. The marketplace lets users discover pre-built contracts for common business processes, deploy them instantly to the hosted platform, and contribute their own.

**Repo:** Both. Public repo gets the registry format and CLI publishing. Private repo gets the marketplace web UI, search, and deployment integration.

---

## What "done" means

1. Contract template format: a packaged contract with metadata (name, description, category, author, version, required facts, entity summary)
2. Registry: searchable catalog of contract templates with categories, tags, ratings
3. Publishing: `tenor publish` uploads a contract template to the registry
4. Discovery: web UI to browse, search, preview contract templates
5. One-click deploy: select a template, configure facts/sources, deploy to the hosted platform
6. Community contributions: anyone can publish, templates are reviewed before listing
7. Template versioning: templates can be updated, users can pin or upgrade

---

## Part A: Public Repo — Template Format and CLI (~/src/riverline/tenor)

### A1: Template manifest

Define `tenor-template.toml`:

```toml
[template]
name = "escrow-release"
version = "1.0.0"
description = "Escrow release workflow with compliance approval and delivery confirmation"
author = "Riverline Labs"
license = "Apache-2.0"
category = "finance"
tags = ["escrow", "compliance", "delivery", "payments"]

[template.metadata]
entities = ["EscrowAccount", "DeliveryRecord"]
personas = ["buyer", "seller", "compliance_officer", "escrow_agent"]
facts_count = 8
flows_count = 3

[template.requirements]
tenor_version = ">=1.0.0"

[template.configuration]
# Facts that the deployer must configure (sources, defaults)
required_sources = ["payment_service", "delivery_service"]

[[template.screenshots]]
path = "screenshots/dashboard.png"
caption = "Contract dashboard showing escrow state"
```

The template package is a directory:

```
escrow-release/
  tenor-template.toml
  contract/
    escrow.tenor       — the contract source
    types.tenor        — shared types (if any)
  examples/
    test-facts.json    — example fact values for simulation
  screenshots/         — optional preview images
  README.md            — detailed description, setup guide
```

### A2: Template packaging

`tenor pack` creates a distributable template archive:

```
tenor pack [OPTIONS]

OPTIONS:
  --output <file>     Output archive (default: <name>-<version>.tenor-template.tar.gz)
```

The command:

1. Reads `tenor-template.toml`
2. Elaborates the contract (validates it compiles)
3. Includes the interchange bundle in the archive
4. Computes archive hash for integrity
5. Produces `<name>-<version>.tenor-template.tar.gz`

### A3: Template publishing

`tenor publish` uploads a template to the registry:

```
tenor publish [OPTIONS]

OPTIONS:
  --registry <url>     Registry URL (default: https://registry.tenor.dev)
  --token <token>      Auth token for publishing
```

The command:

1. Packs the template (if not already packed)
2. Validates the template manifest
3. Uploads to the registry API
4. Reports: "Published escrow-release@1.0.0 to registry.tenor.dev"

### A4: Template search CLI

```
tenor search <query> [OPTIONS]

OPTIONS:
  --category <cat>     Filter by category
  --tag <tag>          Filter by tag
  --registry <url>     Registry URL
```

Returns a list of matching templates with name, version, description, author.

### A5: Template install CLI

```
tenor install <template-name> [OPTIONS]

OPTIONS:
  --version <ver>      Specific version (default: latest)
  --output <dir>       Output directory
  --registry <url>     Registry URL
```

Downloads and unpacks a template to the output directory.

### Acceptance criteria — Part A

- [ ] Template manifest format defined (tenor-template.toml)
- [ ] `tenor pack` creates archive with contract + metadata + interchange bundle
- [ ] `tenor publish` uploads to registry API
- [ ] `tenor search` queries registry
- [ ] `tenor install` downloads and unpacks
- [ ] Template includes pre-elaborated interchange bundle
- [ ] Tests for pack/unpack round-trip

---

## Part B: Private Repo — Registry and Marketplace (~/src/riverline/tenor-platform)

### B1: Registry API

```
POST   /api/v1/registry/templates              — publish template
GET    /api/v1/registry/templates               — list/search templates
GET    /api/v1/registry/templates/{name}        — get template metadata
GET    /api/v1/registry/templates/{name}/{ver}  — get specific version
GET    /api/v1/registry/templates/{name}/{ver}/download  — download archive
DELETE /api/v1/registry/templates/{name}/{ver}  — unpublish (author only)
POST   /api/v1/registry/templates/{name}/rate   — rate a template
```

Search supports: full-text search on name + description, category filter, tag filter, sort by (downloads, rating, newest).

### B2: Registry storage

```
Template {
  name: String (unique)
  author_org_id: UUID
  latest_version: String
}

TemplateVersion {
  template_name: String
  version: String (semver)
  description: String
  category: String
  tags: Vec<String>
  metadata: TemplateMetadata  // entities, personas, facts_count, etc.
  archive_hash: String
  archive_url: String         // S3 or local storage
  bundle_etag: String         // for quick contract identity check
  downloads: u64
  published_at: DateTime
  status: ReviewStatus        // pending, approved, rejected
}

TemplateRating {
  template_name: String
  org_id: UUID
  rating: u8     // 1-5
  created_at: DateTime
}
```

### B3: Review workflow

Published templates go through review before appearing in search:

1. Author publishes → status = `pending`
2. Platform operator reviews (admin dashboard) → `approved` or `rejected`
3. Approved templates appear in search results
4. Rejected templates: author notified with reason

For v1, review is manual via the admin dashboard. Automated checks (elaboration success, no malicious content) can be added later.

### B4: Marketplace web UI

A web application (separate from the admin dashboard) for end users:

**Browse page:**

- Category navigation (sidebar): finance, compliance, supply chain, HR, healthcare, etc.
- Search bar with auto-complete
- Template cards: name, description, author, rating, download count
- Sort: most popular, highest rated, newest

**Template detail page:**

- Full description (README rendered as markdown)
- Screenshots/previews
- Contract summary: entities, personas, facts, flows (from metadata)
- Version history
- Rating and reviews
- "Deploy" button → deployment wizard

**Deployment wizard:**

1. Select organization (from user's orgs)
2. Configure sources: for each `required_sources`, prompt for connection details (protocol fields)
3. Configure persona mappings: map API keys to contract personas
4. Review: show contract summary, configured sources, persona mappings
5. Deploy: calls the hosted platform's deployment API with the template's interchange bundle + configuration
6. Result: live endpoint URL, link to the Automatic UI

### B5: One-click deploy from CLI

```
tenor deploy <template-name> --org <org-id> [OPTIONS]

OPTIONS:
  --version <ver>       Template version
  --config <file>       Configuration file (sources, persona mappings)
  --registry <url>      Registry URL
  --platform <url>      Platform URL
  --token <token>       Platform auth token
```

Combines `tenor install` + configure + deploy into one command.

### Acceptance criteria — Part B

- [ ] Registry API: publish, search, download, rate
- [ ] Registry storage: templates, versions, ratings
- [ ] Review workflow: pending → approved/rejected
- [ ] Marketplace web UI: browse, search, detail, deploy wizard
- [ ] One-click deploy from web and CLI
- [ ] Template versioning (multiple versions, latest resolution)
- [ ] Tests: publish → search → deploy round-trip

---

## Final Report

```
## Phase 11: Marketplace — COMPLETE

### Public repo
- Template format: tenor-template.toml
- CLI: pack, publish, search, install, deploy
- Tests: [N] passing

### Private repo
- Registry API: [N] endpoints
- Registry storage: templates, versions, ratings
- Review workflow: manual via admin dashboard
- Marketplace web UI: browse, search, detail, deploy wizard
- One-click deploy: web and CLI
- Tests: [N] passing

### Categories
- [list of initial categories]

### Commits
Public: [list]
Private: [list]
```

Phase 11 is done when contracts can be published, discovered, previewed, and deployed with one click, and every checkbox above is checked. Not before.
