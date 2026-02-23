# Tenor Slack Bot Example

Slack bot for interacting with Tenor contracts via slash commands. List contracts, evaluate with fact input modals, and get plain-language explanations -- all from Slack.

## Architecture

```
Slack Workspace
  |
  +-- Slack App (Socket Mode)
        |
        +-- Bolt Handlers
              |
              +-- TenorClient SDK
                    |
                    +-- tenor serve (HTTP API)
                          |
                          +-- Contract (.tenor file)
```

Socket Mode is used for local development -- no public URL or ngrok needed.

## Slack App Setup

### 1. Create a Slack App

Go to [api.slack.com/apps](https://api.slack.com/apps) and create a new app. You can use the manifest below to configure everything at once.

### 2. App Manifest

When creating the app, choose "From a manifest" and paste this YAML:

```yaml
display_information:
  name: Tenor Contract Bot
  description: Interact with Tenor contracts via slash commands
  background_color: "#1a1a2e"

features:
  bot_user:
    display_name: Tenor Bot
    always_online: false
  slash_commands:
    - command: /tenor-list
      description: List available contracts
      usage_hint: ""
      should_escape: false
    - command: /tenor-eval
      description: Evaluate a contract with facts
      usage_hint: "[contract_id]"
      should_escape: false
    - command: /tenor-explain
      description: Explain a contract in plain language
      usage_hint: "[contract_id]"
      should_escape: false

oauth_config:
  scopes:
    bot:
      - commands
      - chat:write

settings:
  interactivity:
    is_enabled: true
  socket_mode_enabled: true
  org_deploy_enabled: false
  token_rotation_enabled: false
```

### 3. Install to Workspace

After creating the app, install it to your workspace from the "Install App" page.

### 4. Get Tokens

You need three values:

| Token | Where to find it |
|-------|-----------------|
| `SLACK_BOT_TOKEN` | OAuth & Permissions > Bot User OAuth Token (`xoxb-...`) |
| `SLACK_SIGNING_SECRET` | Basic Information > App Credentials > Signing Secret |
| `SLACK_APP_TOKEN` | Basic Information > App-Level Tokens > Generate (`xapp-...`) with `connections:write` scope |

## Environment Variables

```bash
export SLACK_BOT_TOKEN="xoxb-your-bot-token"
export SLACK_SIGNING_SECRET="your-signing-secret"
export SLACK_APP_TOKEN="xapp-your-app-token"
export TENOR_URL="http://localhost:8080"  # Optional, defaults to http://localhost:8080
```

## Quick Start

1. **Start the Tenor evaluator:**

```bash
# From the repo root
cargo run -p tenor-cli -- serve --port 8080 domains/saas/saas_subscription.tenor
```

2. **Install dependencies:**

```bash
cd examples/slack-bot
npm install
```

3. **Set environment variables** (see above)

4. **Start the bot:**

```bash
npm start
# or: node --experimental-strip-types src/app.ts
```

## Usage

### `/tenor-list`

Lists all contracts loaded in the evaluator. Shows contract ID, fact count, operation count, and flow count.

### `/tenor-eval saas_subscription`

Opens a modal dialog with input fields for each fact in the contract. Fill in fact values and click "Evaluate" to see verdicts.

Values are parsed as JSON when possible. Plain values are interpreted as:
- `true`/`false` as booleans
- Numbers as integers or floats
- Everything else as strings (useful for enum values)

### `/tenor-explain saas_subscription`

Returns a plain-language explanation of the contract including:
- Contract summary (entities, personas, rules, operations, flows)
- Decision flow narrative
- Fact inventory
- Risk and coverage notes

## Extending

- **Add new commands:** Register additional handlers in `handlers.ts`
- **Custom formatting:** Modify Slack Block Kit templates in handler functions
- **Approval workflows:** Use Slack interactive messages to create contract evaluation approval flows
- **Notifications:** Post evaluation results to channels when specific verdicts are produced

## Note

This is a reference implementation showing the Slack integration pattern. For production use, add error logging, rate limiting, and proper Slack app distribution configuration.
