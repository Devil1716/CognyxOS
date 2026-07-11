import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';

type IconName =
  | 'activity' | 'bell' | 'box' | 'brain' | 'calendar' | 'chevron' | 'clock'
  | 'cpu' | 'database' | 'file' | 'grid' | 'heart' | 'home' | 'layers' | 'menu'
  | 'monitor' | 'network' | 'package' | 'play' | 'plus' | 'refresh' | 'search'
  | 'settings' | 'shield' | 'sliders' | 'terminal' | 'tool' | 'users' | 'zap';

function Icon({ name, size = 18 }: { name: IconName; size?: number }): React.JSX.Element {
  const paths: Record<IconName, React.JSX.Element> = {
    activity: <><path d="M3 12h4l2-7 4 14 2-7h6" /></>,
    bell: <><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4" /></>,
    box: <><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" /><path d="m4 7.5 8 4.5 8-4.5M12 12v9" /></>,
    brain: <><path d="M9.5 4A3.5 3.5 0 0 0 6 7.5 3 3 0 0 0 4 13a3.4 3.4 0 0 0 2 6h3.5V4ZM14.5 4A3.5 3.5 0 0 1 18 7.5a3 3 0 0 1 2 5.5 3.4 3.4 0 0 1-2 6h-3.5V4Z" /><path d="M9.5 8H7M14.5 8H17M9.5 14H7M14.5 14H17" /></>,
    calendar: <><rect x="4" y="5" width="16" height="15" rx="2" /><path d="M8 3v4M16 3v4M4 10h16" /></>,
    chevron: <path d="m7 10 5 5 5-5" />,
    clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
    cpu: <><rect x="6" y="6" width="12" height="12" rx="2" /><path d="M9 1v5M15 1v5M9 18v5M15 18v5M18 9h5M18 15h5M1 9h5M1 15h5" /></>,
    database: <><ellipse cx="12" cy="5" rx="7" ry="3" /><path d="M5 5v7c0 1.7 3.1 3 7 3s7-1.3 7-3V5M5 12v7c0 1.7 3.1 3 7 3s7-1.3 7-3v-7" /></>,
    file: <><path d="M6 3h8l4 4v14H6z" /><path d="M14 3v5h5M9 13h6M9 17h5" /></>,
    grid: <><rect x="4" y="4" width="6" height="6" rx="1" /><rect x="14" y="4" width="6" height="6" rx="1" /><rect x="4" y="14" width="6" height="6" rx="1" /><rect x="14" y="14" width="6" height="6" rx="1" /></>,
    heart: <><path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.9-8.6a5.5 5.5 0 0 0-.1-7.8Z" /></>,
    home: <><path d="m3 11 9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1v-9Z" /></>,
    layers: <><path d="m12 3 9 5-9 5-9-5 9-5ZM3 12l9 5 9-5M3 16l9 5 9-5" /></>,
    menu: <><path d="M4 7h16M4 12h16M4 17h16" /></>,
    monitor: <><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4" /></>,
    network: <><circle cx="12" cy="5" r="2" /><circle cx="5" cy="18" r="2" /><circle cx="19" cy="18" r="2" /><path d="m10.7 6.5-4.4 9M13.3 6.5l4.4 9M7 18h10" /></>,
    package: <><path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" /><path d="m4 7.5 8 4.5 8-4.5M12 12v9" /></>,
    play: <><circle cx="12" cy="12" r="9" /><path d="m10 8 6 4-6 4Z" /></>,
    plus: <path d="M12 5v14M5 12h14" />,
    refresh: <><path d="M20 11a8 8 0 1 0 1 5" /><path d="M20 4v7h-7" /></>,
    search: <><circle cx="11" cy="11" r="6" /><path d="m16 16 4 4" /></>,
    settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.1 2.1-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5v.2h-3v-.2a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1-2.1-2.1.1-.1A1.7 1.7 0 0 0 7 15a1.7 1.7 0 0 0-1.5-1H5.3v-3h.2A1.7 1.7 0 0 0 7 10a1.7 1.7 0 0 0-.3-1.9l-.1-.1 2.1-2.1.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5v-.2h3v.2a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1 2.1 2.1-.1.1A1.7 1.7 0 0 0 19.4 10a1.7 1.7 0 0 0 1.5 1h.2v3h-.2a1.7 1.7 0 0 0-1.5 1Z" /></>,
    shield: <><path d="M12 3 20 6v5c0 5-3.5 8.5-8 10-4.5-1.5-8-5-8-10V6l8-3Z" /><path d="m8.5 12 2.2 2.2 4.8-5" /></>,
    sliders: <><path d="M4 7h16M4 17h16M8 4v6M16 14v6" /></>,
    terminal: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m7 9 3 3-3 3M13 15h4" /></>,
    tool: <><path d="M14.7 6.3a5 5 0 0 0-6.5 6.5L3 18l3 3 5.2-5.2a5 5 0 0 0 6.5-6.5l-3.3 3.3-2.8-2.8 3.1-3.5Z" /></>,
    users: <><circle cx="9" cy="8" r="3" /><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6M16 5.5a3 3 0 0 1 0 5M17 14c2.3.4 4 2.4 4 5" /></>,
    zap: <path d="m13 2-9 12h7l-1 8 9-12h-7l1-8Z" />,
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>;
}

const navGroups = [
  ['Overview', [['home', 'Dashboard'], ['activity', 'System Health'], ['zap', 'Events'], ['grid', 'Services'], ['package', 'Plugins'], ['users', 'Agents'], ['tool', 'Tools'], ['layers', 'Models'], ['database', 'Memory'], ['monitor', 'Desktop']]],
  ['Operations', [['calendar', 'Tasks'], ['calendar', 'Scheduler'], ['network', 'IPC Monitor'], ['file', 'Logs'], ['activity', 'Metrics']]],
  ['Developer', [['terminal', 'Console'], ['heart', 'Doctor'], ['settings', 'Settings']]],
] as const;

const services = ['Runtime', 'Event Bus', 'IPC Server', 'Scheduler', 'Service Registry', 'Configuration Service'];
const sparkPaths = ['M1 25 8 25 12 16 17 24 23 10 28 19 34 8 39 24 46 21 52 25 59 17 65 24 72 22 78 27', 'M1 25 8 25 14 18 20 24 28 10 34 19 40 8 48 24 55 21 62 25 69 17 76 24', 'M1 25 9 25 14 21 21 11 28 23 34 7 40 18 49 9 55 23 62 21 69 27 77 20'];

function Sparkline({ color = 'green', index = 0 }: { color?: string; index?: number }): React.JSX.Element {
  return <svg className={'spark ' + color} viewBox="0 0 80 30" preserveAspectRatio="none"><path d={sparkPaths[index % sparkPaths.length]} /></svg>;
}

function Card({ icon, tone, title, value, detail }: { icon: IconName; tone: string; title: string; value: string; detail: React.ReactNode }): React.JSX.Element {
  return <article className="status-card"><span className={'icon-badge ' + tone}><Icon name={icon} size={25} /></span><p>{title}</p><strong className={tone === 'green' ? 'success' : ''}>{value}</strong><small>{detail}</small></article>;
}

function App(): React.JSX.Element {
  return <div className="app-shell">
    <aside className="sidebar">
      <a className="brand" href="#top"><span className="brand-mark">C</span><b>Cognyx<span>OS</span></b></a>
      <nav>{navGroups.map(([title, items]) => <section className="nav-group" key={title}><h2>{title}</h2>{items.map(([icon, label]) => <a key={label} className={label === 'Dashboard' ? 'active' : ''} href={'#' + label.toLowerCase().replace(' ', '-')}><Icon name={icon as IconName} size={16} />{label}</a>)}</section>)}</nav>
      <div className="sidebar-footer"><Icon name="brain" size={18} /><div><b>CognyxOS</b><small>Intelligent Operating Environment</small><small>© 2025 Cognyx Labs</small></div></div>
    </aside>
    <main id="top">
      <header className="topbar"><button className="menu-button"><Icon name="menu" /></button><h1>Dashboard</h1><span className="online"><i />System Online</span><div className="top-actions"><button className="runtime"><Icon name="terminal" size={16} /> Runtime v0.2.0</button><button><Icon name="settings" /></button><button className="notification"><Icon name="bell" /><em>3</em></button><button className="avatar">DA</button><Icon name="chevron" size={16} /></div></header>
      <div className="content">
        <section className="greeting"><div><h2>Good morning, Developer 👋</h2><p>Here's what's happening with your system.</p></div><div className="date"><Icon name="clock" size={14} /> 10:24 AM <span>•</span> May 20, 2025</div></section>
        <div className="dashboard-grid">
          <div className="left-column">
            <section className="status-row"><Card icon="shield" tone="green" title="System Status" value="Healthy" detail="All systems operational" /><Card icon="clock" tone="blue" title="Uptime" value="00:02:47" detail="Since last startup" /><Card icon="box" tone="purple" title="Services" value="12" detail={<><span className="success">12 running</span> · <span className="danger">0 stopped</span></>} /><Card icon="activity" tone="orange" title="Events (24h)" value="1,248" detail={<><span className="danger">124 errors</span> · <span className="blue-text">0 dropped</span></>} /><Card icon="package" tone="green" title="Plugins" value="3" detail={<><span className="success">3 loaded</span> · <span className="danger">0 failed</span></>} /></section>
            <section className="panel resources"><div className="panel-heading"><h3>System Resources</h3><span className="live"><i /> Real-time</span></div><div className="resource-row">{[['CPU', '8%', '3.2 GHz', 'green'], ['RAM', '38%', '6.1 / 16 GB', 'blue'], ['Memory Store', '120 MB', 'SQLite (WAL)', 'purple'], ['Disk', '18%', '85 / 475 GB', 'orange'], ['Network', '↟ 2.4 MB/s', '↡ 1.1 MB/s', 'blue']].map(([name, value, sub, color], i) => <div className="resource" key={name}><span>{name}</span><div className={'gauge ' + color} style={{ '--percent': i === 2 ? '46%' : ['8%', '38%', '46%', '18%', '38%'][i] } as React.CSSProperties}><b>{value}</b></div><small>{sub}</small><Sparkline color={color} index={i} /></div>)}</div></section>
            <section className="panel services"><div className="panel-heading"><h3>Services</h3><a href="#services">View all services</a></div><div className="service-table"><div className="table-head"><span>Service</span><span>Status</span><span>Uptime</span><span>CPU</span><span>Memory</span><span>Health</span></div>{services.map((service, i) => <div className="service-row" key={service}><span><Icon name={i < 3 ? 'box' : 'cpu'} size={14} />{service}</span><span className="running"><i />Running</span><span>00:02:{47 - i}</span><span>{[1.2, .8, 1.5, .6, .4, .3][i]}% <Sparkline index={i} /></span><span>{[45, 32, 38, 28, 22, 18][i]} MB</span><span className="health">100% <b /></span></div>)}</div></section>
          </div>
          <aside className="right-column"><section className="panel events"><div className="panel-heading"><h3>⌁Recent Events</h3><a href="#events">View all</a></div>{[['play', 'green', 'RuntimeStarted', 'Runtime has started successfully'], ['box', 'blue', 'ServiceRegistered', "Service 'EventBus' registered"], ['package', 'purple', 'PluginLoaded', "Plugin 'SamplePlugin' loaded successfully"], ['settings', 'orange', 'ConfigurationLoaded', 'Configuration loaded from config/runtime.yaml'], ['network', 'blue', 'IPCChannelReady', 'IPC channel established on port 50051']].map(([icon, tone, title, text], i) => <div className="event" key={title}><span className={'event-icon ' + tone}><Icon name={icon as IconName} size={17} /></span><div><b>{title}</b><p>{text}</p></div><time>10:24:{31 - i} AM</time></div>)}</section></aside>
        </div>
        <div className="middle-row"><section className="panel metrics"><div className="panel-heading"><h3>System Metrics</h3><button className="period">1H <Icon name="chevron" size={13} /></button></div><div className="legend"><span className="green-dot">CPU %</span><span className="blue-dot">RAM %</span><span className="purple-dot">Events/s</span></div><svg className="chart" viewBox="0 0 600 190" preserveAspectRatio="none"><g className="grid-lines"><path d="M35 15H580M35 55H580M35 95H580M35 135H580M35 175H580" /></g><path className="chart-blue" d="M35 56 C55 45 55 63 73 50 S93 54 110 45 S135 60 148 45 S172 53 183 65 S205 58 220 59 S240 51 255 59 S275 54 291 65 S315 61 330 57 S350 68 365 59 S385 63 402 62 S422 58 437 66 S457 59 472 68 S490 61 506 65 S527 55 544 63 S562 55 580 67" /><path className="chart-purple" d="M35 120 C55 82 58 120 75 105 S95 130 110 108 S132 82 148 108 S170 130 184 110 S204 93 220 115 S241 139 255 120 S274 80 291 101 S311 130 330 115 S351 124 365 104 S385 115 402 85 S421 38 437 95 S458 116 472 124 S489 140 506 107 S528 82 544 95 S561 120 580 112" /><path className="chart-green" d="M35 170 45 162 55 169 66 171 77 166 88 172 99 162 110 171 121 169 132 172 143 164 154 170 165 174 176 170 187 165 198 174 209 172 220 168 231 173 242 174 253 165 264 171 275 174 286 164 297 173 308 160 319 174 330 171 341 174 352 166 363 172 374 160 385 171 396 165 407 174 418 160 429 170 440 164 451 174 462 168 473 172 484 162 495 170 506 164 517 174 528 166 539 172 550 164 561 173 580 170" /><g className="axis"><text x="0" y="18">100%</text><text x="8" y="58">75%</text><text x="8" y="98">50%</text><text x="8" y="138">25%</text><text x="14" y="178">0%</text><text x="35" y="189">09:24</text><text x="135" y="189">09:36</text><text x="235" y="189">09:48</text><text x="335" y="189">10:00</text><text x="435" y="189">10:12</text><text x="548" y="189">10:24</text></g></svg></section></div>
        <section className="bottom-row"><section className="panel quick"><h3>Quick Actions</h3><div>{[['refresh', 'Restart', 'Runtime'], ['file', 'Reload', 'Config'], ['file', 'View Logs', ''], ['brain', 'Run Doctor', '']].map(([icon, line1, line2]) => <button key={line1}><Icon name={icon as IconName} size={24} /><span>{line1}<br />{line2}</span></button>)}</div></section><section className="panel scheduler"><h3>Scheduler Overview</h3><div className="donut"><b>44<small>Total Tasks</small></b></div><ul><li><i className="blue-fill" />18 <span>Queued</span></li><li><i className="green-fill" />4 <span>Running</span></li><li><i className="red-fill" />0 <span>Failed</span></li><li><i className="gray-fill" />22 <span>Completed</span></li></ul></section><section className="panel tasks"><h3>Next Tasks</h3>{[['Cleanup Temp Files', 'In 2m 15s'], ['Memory Index Sync', 'In 5m 30s'], ['Plugin Health Check', 'In 10m 00s']].map(([task, when]) => <div key={task}><Icon name="calendar" size={14} /><span>{task}</span><small>{when}</small></div>)}<a href="#schedule">View full schedule →</a></section><section className="panel shortcuts"><h3>Shortcuts</h3>{[['terminal', 'Open Developer Console'], ['file', 'View Architecture Docs'], ['network', 'Visit API Documentation']].map(([icon, label]) => <a href="#shortcut" key={label}><Icon name={icon as IconName} size={17} />{label}</a>)}</section></section>
      </div>
      <footer><span>Environment: <b>Development</b></span><span>Platform: Windows 11 (x64)</span><span>Store: SQLite (WAL)</span><span>Local Time: 10:24:31 AM</span><span className="success"><i /> All systems operational</span></footer>
    </main>
  </div>;
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
