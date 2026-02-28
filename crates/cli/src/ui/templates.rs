/// Generate package.json content for the React UI project.
pub(super) fn package_json(title: &str) -> String {
    let name = to_kebab(title);
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "scripts": {{
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  }},
  "dependencies": {{
    "react": "^19.2.0",
    "react-dom": "^19.2.0",
    "react-router-dom": "^7.13.0"
  }},
  "devDependencies": {{
    "@types/react": "^19.2.0",
    "@types/react-dom": "^19.2.0",
    "@vitejs/plugin-react": "^5.1.0",
    "typescript": "^5.9.0",
    "vite": "^6.4.0"
  }}
}}
"#,
        name = name,
    )
}

/// Generate tsconfig.json content.
pub(super) fn tsconfig_json() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
"#
    .to_string()
}

/// Generate vite.config.ts content.
pub(super) fn vite_config() -> String {
    r#"import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});
"#
    .to_string()
}

/// Generate public/index.html content.
pub(super) fn index_html(title: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
        title = title,
    )
}

/// Generate src/main.tsx content.
pub(super) fn main_tsx() -> String {
    r#"import React from 'react';
import ReactDOM from 'react-dom/client';
import './styles.css';
import App from './App.tsx';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
"#
    .to_string()
}

/// Generate src/styles.css with global reset and responsive layout styles.
pub(super) fn global_css() -> String {
    r#"/* Global reset and base styles */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html {
  font-size: 16px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

body {
  font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
  color: #0f172a;
  background-color: #f8fafc;
}

/* Responsive container */
.app-container {
  display: flex;
  min-height: 100vh;
}

.sidebar {
  width: 260px;
  flex-shrink: 0;
  background: #ffffff;
  border-right: 1px solid #e2e8f0;
  padding: 24px 16px;
}

.main-content {
  flex: 1;
  padding: 24px 32px;
  max-width: 1200px;
}

/* Card component */
.card {
  background: #ffffff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

/* Responsive */
@media (max-width: 768px) {
  .app-container { flex-direction: column; }
  .sidebar { width: 100%; border-right: none; border-bottom: 1px solid #e2e8f0; }
  .main-content { padding: 16px; }
}
"#
    .to_string()
}

/// Generate src/App.tsx content with React Router routes.
pub(super) fn app_tsx(contract_id: &str, title: &str) -> String {
    format!(
        r#"import {{ BrowserRouter, Routes, Route }} from 'react-router-dom';
import {{ Layout }} from './components/Layout.tsx';
import {{ Dashboard }} from './components/Dashboard.tsx';
import {{ EntityList }} from './components/EntityList.tsx';
import {{ EntityDetail }} from './components/EntityDetail.tsx';
import {{ InstanceDetail }} from './components/InstanceDetail.tsx';
import {{ ActionSpace }} from './components/ActionSpace.tsx';
import {{ FlowExecution }} from './components/FlowExecution.tsx';
import {{ FlowHistory }} from './components/FlowHistory.tsx';

// Contract: {contract_id}
// Title: {title}

export default function App() {{
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={{<Layout />}}>
          <Route index element={{<Dashboard />}} />
          <Route path="entities" element={{<EntityList />}} />
          <Route path="entities/:id" element={{<EntityDetail />}} />
          <Route path="entities/:id/:instanceId" element={{<InstanceDetail />}} />
          <Route path="actions" element={{<ActionSpace />}} />
          <Route path="flows/:id" element={{<FlowExecution />}} />
          <Route path="history" element={{<FlowHistory />}} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}}
"#,
        contract_id = contract_id,
        title = title,
    )
}

/// Generate src/components/Layout.tsx with sidebar navigation.
pub(super) fn layout_tsx(title: &str) -> String {
    format!(
        r#"import {{ Outlet, NavLink }} from 'react-router-dom';
import {{ theme }} from '../theme.ts';
import {{ useState }} from 'react';

const navStyle = (isActive: boolean): React.CSSProperties => ({{
  display: 'block',
  padding: '8px 16px',
  textDecoration: 'none',
  borderRadius: '6px',
  color: isActive ? theme.colors.primary : theme.colors.textPrimary,
  backgroundColor: isActive ? theme.colors.primaryLight : 'transparent',
  fontWeight: isActive ? 600 : 400,
  marginBottom: '4px',
}});

const NAV_LINKS = [
  {{ to: '/', label: 'Dashboard', end: true }},
  {{ to: '/entities', label: 'Entities', end: false }},
  {{ to: '/actions', label: 'Actions', end: false }},
  {{ to: '/history', label: 'History', end: false }},
];

export function Layout() {{
  const [persona, setPersona] = useState('');

  return (
    <div style={{{{ display: 'flex', minHeight: '100vh', fontFamily: theme.fonts.body }}}}>
      {{/* Sidebar */}}
      <nav style={{{{
        width: '220px',
        backgroundColor: theme.colors.surface,
        borderRight: `1px solid ${{theme.colors.border}}`,
        padding: '16px',
        flexShrink: 0,
      }}}}>
        <div style={{{{ marginBottom: '24px' }}}}>
          <h1 style={{{{ fontSize: '16px', fontWeight: 700, color: theme.colors.primary, margin: 0 }}}}>
            {title}
          </h1>
        </div>
        {{NAV_LINKS.map((link) => (
          <NavLink
            key={{link.to}}
            to={{link.to}}
            end={{link.end}}
            style={{({{ isActive }}) => navStyle(isActive)}}
          >
            {{link.label}}
          </NavLink>
        ))}}
      </nav>

      {{/* Main content */}}
      <div style={{{{ flex: 1, display: 'flex', flexDirection: 'column' }}}}>
        {{/* Header */}}
        <header style={{{{
          padding: '12px 24px',
          borderBottom: `1px solid ${{theme.colors.border}}`,
          backgroundColor: theme.colors.surface,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          gap: '12px',
        }}}}>
          <label style={{{{ fontSize: '14px', color: theme.colors.textSecondary }}}}>
            Persona:
          </label>
          <select
            value={{persona}}
            onChange={{(e) => setPersona(e.target.value)}}
            style={{{{
              padding: '6px 10px',
              borderRadius: '6px',
              border: `1px solid ${{theme.colors.border}}`,
              fontSize: '14px',
            }}}}
          >
            <option value="">Select persona...</option>
          </select>
        </header>

        {{/* Page content */}}
        <main style={{{{ flex: 1, padding: '24px', backgroundColor: theme.colors.background }}}}>
          <Outlet />
        </main>
      </div>
    </div>
  );
}}
"#,
        title = title,
    )
}

/// Convert a title string to kebab-case for use as a package name.
fn to_kebab(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
