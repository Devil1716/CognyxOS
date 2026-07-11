import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { loadConfig } from '@cognyx/config';
import { JsonLogger } from '@cognyx/logging';
import './styles.css';

const config = loadConfig();
new JsonLogger().log('INFO', 'desktop_shell_initialized', { environment: config.environment });

function FoundationShell(): React.JSX.Element {
  return (
    <main>
      <h1>CognyxOS</h1>
      <p>Engineering foundation initialized.</p>
    </main>
  );
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <FoundationShell />
  </StrictMode>,
);
