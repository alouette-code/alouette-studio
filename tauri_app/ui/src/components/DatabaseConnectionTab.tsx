import React, { useEffect, useState } from 'react';
import {
    Database,
    Lock,
    Server,
    User,
    Cable,
    ArrowLeft,
    FolderOpen,
    Trash2,
    Plug,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import AlouetteSelect from './AlouetteSelect';

export type DbDriver =
    | 'postgres'
    | 'mysql'
    | 'sqlite'
    | 'duckdb'
    | 'mongodb'
    | 'redis'
    | 'supabase'
    | 'sled'
    | 'redb';

export interface DbConnection {
    id: string;
    name: string;
    uri: string;
    driver: DbDriver;
}

interface Props {
    onConnect: (uri: string) => void;
}

const DRIVERS: { id: DbDriver; label: string; subtitle: string; defaultPort?: string }[] = [
    { id: 'postgres', label: 'PostgreSQL', subtitle: 'Relational SQL', defaultPort: '5432' },
    { id: 'mysql', label: 'MySQL / MariaDB', subtitle: 'Relational SQL', defaultPort: '3306' },
    { id: 'sqlite', label: 'SQLite', subtitle: 'Embedded SQL file' },
    { id: 'duckdb', label: 'DuckDB', subtitle: 'Analytical OLAP file' },
    { id: 'mongodb', label: 'MongoDB', subtitle: 'Document NoSQL', defaultPort: '27017' },
    { id: 'redis', label: 'Redis', subtitle: 'In-memory KV store', defaultPort: '6379' },
    { id: 'supabase', label: 'Supabase', subtitle: 'Managed Postgres', defaultPort: '5432' },
    { id: 'sled', label: 'Sled KV', subtitle: 'Embedded Rust KV dir' },
    { id: 'redb', label: 'Redb', subtitle: 'Embedded Rust DB file' },
];

function maskUriSecrets(raw: string): string {
    try {
        if (!raw.includes('://')) return raw;
        const parsed = new URL(raw);
        if (parsed.password) {
            parsed.password = '••••••';
        }
        return parsed.toString();
    } catch {
        return raw.replace(/:([^:@/]+)@/, ':••••••@');
    }
}

export default function DatabaseConnectionTab({ onConnect }: Props) {
    const [step, setStep] = useState<1 | 2>(1);
    const [driver, setDriver] = useState<DbDriver>('postgres');
    const [savedConnections, setSavedConnections] = useState<DbConnection[]>([]);

    // Connection form state
    const [name, setName] = useState('');
    const [host, setHost] = useState('localhost');
    const [port, setPort] = useState('5432');
    const [user, setUser] = useState('postgres');
    const [password, setPassword] = useState('');
    const [database, setDatabase] = useState('');
    const [filePath, setFilePath] = useState('');

    // Advanced / Cloud Mode state
    const [connectionMode, setConnectionMode] = useState<'standard' | 'advanced'>('standard');
    const [customUri, setCustomUri] = useState('');
    const [authType, setAuthType] = useState('basic');
    const [sslCert, setSslCert] = useState('');
    const [rememberCredentials, setRememberCredentials] = useState(true);

    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        try {
            const raw = JSON.parse(localStorage.getItem('db_connections') || '[]');
            if (Array.isArray(raw)) {
                setSavedConnections(raw);
            }
        } catch {
            setSavedConnections([]);
        }
    }, []);

    const isFileDriver = driver === 'sqlite' || driver === 'duckdb' || driver === 'sled' || driver === 'redb';

    const handleSelectDriver = (selected: DbDriver) => {
        setDriver(selected);
        const meta = DRIVERS.find((d) => d.id === selected);
        if (meta?.defaultPort) setPort(meta.defaultPort);
        if (selected === 'mysql') setUser('root');
        else if (selected === 'postgres' || selected === 'supabase') setUser('postgres');
        else if (selected === 'redis' || selected === 'mongodb') setUser('');
        setFilePath('');
        setError(null);
        setStep(2);
    };

    const handleBrowseFile = async () => {
        try {
            const selectedPath: string | null = await invoke('open_file_dialog');
            if (selectedPath) {
                setFilePath(selectedPath);
            }
        } catch (err: any) {
            setError(err.toString());
        }
    };

    const handleBrowseCert = async () => {
        try {
            const selectedPath: string | null = await invoke('open_file_dialog');
            if (selectedPath) {
                setSslCert(selectedPath);
            }
        } catch (err: any) {
            setError(err.toString());
        }
    };

    const handleDeleteSaved = (id: string) => {
        const next = savedConnections.filter((c) => c.id !== id);
        setSavedConnections(next);
        localStorage.setItem('db_connections', JSON.stringify(next));
    };

    const handleConnect = async (e?: React.FormEvent) => {
        if (e) e.preventDefault();
        setLoading(true);
        setError(null);

        let uri = '';
        if (!isFileDriver && connectionMode === 'advanced') {
            if (!customUri.trim()) {
                setError('Connection URI is required in Advanced mode.');
                setLoading(false);
                return;
            }
            uri = customUri.trim();
        } else if (isFileDriver) {
            if (!filePath.trim()) {
                setError(`Please specify a ${driver.toUpperCase()} path.`);
                setLoading(false);
                return;
            }
            const clean = filePath.trim();
            uri = clean.includes('://') ? clean : `${driver}://${clean}`;
        } else if (driver === 'redis') {
            const auth = user && password
                ? `${encodeURIComponent(user)}:${encodeURIComponent(password)}@`
                : password
                    ? `:${encodeURIComponent(password)}@`
                    : '';
            const dbPath = database ? `/${database}` : '';
            uri = `redis://${auth}${host}:${port}${dbPath}`;
        } else if (driver === 'mongodb') {
            const auth = user && password
                ? `${encodeURIComponent(user)}:${encodeURIComponent(password)}@`
                : '';
            const dbPath = database ? `/${database}` : '';
            uri = `mongodb://${auth}${host}:${port}${dbPath}`;
        } else {
            if (!database.trim()) {
                setError('Database name is required.');
                setLoading(false);
                return;
            }
            const scheme = driver === 'supabase' ? 'postgres' : driver;
            const auth = user
                ? password
                    ? `${encodeURIComponent(user)}:${encodeURIComponent(password)}@`
                    : `${encodeURIComponent(user)}@`
                : '';
            uri = `${scheme}://${auth}${host}:${port}/${database.trim()}`;
        }

        try {
            const res = await invoke<{ success: boolean; message: string }>('connect_to_db', {
                options: {
                    uri,
                    auth_type: connectionMode === 'advanced' ? authType : 'basic',
                    username: user || null,
                    password: password || null,
                    ssl_cert: connectionMode === 'advanced' && authType === 'x509_cert' ? sslCert : null,
                },
            });

            if (res.success) {
                const storedUri = rememberCredentials ? uri : maskUriSecrets(uri);
                const newConn: DbConnection = {
                    id: crypto.randomUUID(),
                    name:
                        name.trim() ||
                        (isFileDriver ? filePath.split(/[/\\]/).pop()! : database || host) ||
                        'New Connection',
                    uri: storedUri,
                    driver,
                };
                const next = [...savedConnections.filter((c) => c.uri !== storedUri), newConn];
                setSavedConnections(next);
                localStorage.setItem('db_connections', JSON.stringify(next));
                onConnect(uri);
            } else {
                setError(res.message);
            }
        } catch (err: any) {
            setError(err.toString());
        } finally {
            setLoading(false);
        }
    };

    return (
        <div
            style={{
                width: '100%',
                height: '100%',
                display: 'flex',
                flexDirection: 'column',
                backgroundColor: 'var(--bg-primary, #0a0a0c)',
                color: 'var(--text-primary, #e6e6eb)',
                overflowY: 'auto',
                padding: '32px 40px',
            }}
        >
            <div style={{ maxWidth: '880px', width: '100%', margin: '0 auto' }}>
                <div style={{ marginBottom: '28px' }}>
                    <h1
                        style={{
                            fontSize: '22px',
                            fontWeight: 700,
                            margin: '0 0 6px 0',
                            display: 'flex',
                            alignItems: 'center',
                            gap: '10px',
                        }}
                    >
                        <Cable size={24} color="var(--color-accent, #3a86ff)" />
                        Universal Database Studio
                    </h1>
                    <p style={{ color: 'var(--text-muted, #585866)', margin: 0, fontSize: '13px' }}>
                        Connect to SQL, OLAP, Document, or Key-Value databases directly inside Alouette.
                    </p>
                </div>

                {step === 1 && (
                    <div className="animate-in fade-in slide-in-from-bottom-4 duration-300">
                        {savedConnections.length > 0 && (
                            <div style={{ marginBottom: '28px' }}>
                                <h2
                                    style={{
                                        fontSize: '12px',
                                        textTransform: 'uppercase',
                                        letterSpacing: '0.6px',
                                        fontWeight: 700,
                                        marginBottom: '12px',
                                        color: 'var(--text-secondary, #92929e)',
                                    }}
                                >
                                    Saved Connections ({savedConnections.length})
                                </h2>
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                    {savedConnections.map((conn) => (
                                        <div
                                            key={conn.id}
                                            style={{
                                                display: 'flex',
                                                alignItems: 'center',
                                                justifyContent: 'space-between',
                                                padding: '10px 14px',
                                                backgroundColor: 'var(--bg-secondary, #111115)',
                                                border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px',
                                                gap: '12px',
                                            }}
                                        >
                                            <div style={{ minWidth: 0, flex: 1 }}>
                                                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                                                    <span style={{ fontWeight: 600, fontSize: '13px' }}>{conn.name}</span>
                                                    <span
                                                        style={{
                                                            fontSize: '10px',
                                                            textTransform: 'uppercase',
                                                            padding: '2px 6px',
                                                            borderRadius: '4px',
                                                            backgroundColor: 'rgba(58, 134, 255, 0.14)',
                                                            color: 'var(--color-accent, #3a86ff)',
                                                            fontWeight: 600,
                                                        }}
                                                    >
                                                        {conn.driver}
                                                    </span>
                                                </div>
                                                <div
                                                    style={{
                                                        fontSize: '11px',
                                                        color: 'var(--text-muted, #6b7280)',
                                                        fontFamily: 'var(--font-mono)',
                                                        overflow: 'hidden',
                                                        textOverflow: 'ellipsis',
                                                        whiteSpace: 'nowrap',
                                                        marginTop: '2px',
                                                    }}
                                                >
                                                    {maskUriSecrets(conn.uri)}
                                                </div>
                                            </div>
                                            <div style={{ display: 'flex', gap: '6px' }}>
                                                <button
                                                    type="button"
                                                    onClick={() => onConnect(conn.uri)}
                                                    style={{
                                                        backgroundColor: 'var(--color-accent, #3a86ff)',
                                                        color: '#fff',
                                                        border: 'none',
                                                        borderRadius: '6px',
                                                        padding: '6px 12px',
                                                        fontSize: '12px',
                                                        fontWeight: 600,
                                                        cursor: 'pointer',
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                        gap: '5px',
                                                    }}
                                                >
                                                    <Plug size={12} /> Open
                                                </button>
                                                <button
                                                    type="button"
                                                    onClick={() => handleDeleteSaved(conn.id)}
                                                    title="Remove saved connection"
                                                    style={{
                                                        backgroundColor: 'transparent',
                                                        color: 'var(--text-muted)',
                                                        border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '6px',
                                                        padding: '6px 8px',
                                                        cursor: 'pointer',
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                    }}
                                                >
                                                    <Trash2 size={13} />
                                                </button>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            </div>
                        )}

                        <h2
                            style={{
                                fontSize: '12px',
                                textTransform: 'uppercase',
                                letterSpacing: '0.6px',
                                fontWeight: 700,
                                marginBottom: '14px',
                                color: 'var(--text-secondary, #92929e)',
                            }}
                        >
                            Step 1: Select Database Engine
                        </h2>

                        <div
                            style={{
                                display: 'grid',
                                gridTemplateColumns: 'repeat(auto-fill, minmax(165px, 1fr))',
                                gap: '12px',
                            }}
                        >
                            {DRIVERS.map((item) => (
                                <button
                                    key={item.id}
                                    type="button"
                                    onClick={() => handleSelectDriver(item.id)}
                                    style={{
                                        backgroundColor: 'var(--bg-secondary, #111115)',
                                        border: '1px solid var(--border-primary, #22222a)',
                                        borderRadius: '10px',
                                        padding: '20px 16px',
                                        cursor: 'pointer',
                                        transition: 'border-color 0.15s, color 0.15s',
                                        display: 'flex',
                                        flexDirection: 'column',
                                        alignItems: 'center',
                                        gap: '8px',
                                        color: 'var(--text-primary, #e6e6eb)',
                                        textAlign: 'center',
                                    }}
                                >
                                    <Server size={30} color="var(--color-accent, #3a86ff)" />
                                    <span style={{ fontWeight: 600, fontSize: '13px' }}>{item.label}</span>
                                    <span style={{ fontSize: '11px', color: 'var(--text-muted, #6b7280)' }}>
                                        {item.subtitle}
                                    </span>
                                </button>
                            ))}
                        </div>
                    </div>
                )}

                {step === 2 && (
                    <div className="animate-in fade-in slide-in-from-right-8 duration-300">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '20px' }}>
                            <button
                                type="button"
                                onClick={() => setStep(1)}
                                style={{
                                    border: 'none',
                                    color: 'var(--text-secondary)',
                                    cursor: 'pointer',
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '6px',
                                    padding: '6px 12px',
                                    borderRadius: '6px',
                                    backgroundColor: 'rgba(255,255,255,0.05)',
                                }}
                            >
                                <ArrowLeft size={14} /> Back
                            </button>
                            <h2 style={{ fontSize: '14px', fontWeight: 600, margin: 0, color: 'var(--text-secondary, #92929e)' }}>
                                Step 2: Configure {driver.toUpperCase()}
                            </h2>
                        </div>

                        <div
                            style={{
                                backgroundColor: 'var(--bg-secondary, #111115)',
                                border: '1px solid var(--border-primary, #22222a)',
                                borderRadius: '12px',
                                padding: '28px',
                            }}
                        >
                            <form
                                id="db-conn-form"
                                onSubmit={handleConnect}
                                style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}
                            >
                                {error && (
                                    <div
                                        style={{
                                            padding: '12px 16px',
                                            backgroundColor: 'rgba(239, 68, 68, 0.1)',
                                            border: '1px solid rgba(239, 68, 68, 0.25)',
                                            borderRadius: '8px',
                                            color: 'var(--color-danger, #ef4444)',
                                            fontSize: '13px',
                                        }}
                                    >
                                        {error}
                                    </div>
                                )}

                                {!isFileDriver && (
                                    <div
                                        style={{
                                            display: 'flex',
                                            gap: '8px',
                                            backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                            padding: '5px',
                                            borderRadius: '8px',
                                        }}
                                    >
                                        <button
                                            type="button"
                                            onClick={() => setConnectionMode('standard')}
                                            style={{
                                                flex: 1,
                                                padding: '9px',
                                                borderRadius: '6px',
                                                border: 'none',
                                                cursor: 'pointer',
                                                fontSize: '12px',
                                                fontWeight: 600,
                                                backgroundColor:
                                                    connectionMode === 'standard' ? 'var(--color-accent, #3a86ff)' : 'transparent',
                                                color: connectionMode === 'standard' ? '#fff' : 'var(--text-muted, #585866)',
                                            }}
                                        >
                                            Standard (Host / Port)
                                        </button>
                                        <button
                                            type="button"
                                            onClick={() => setConnectionMode('advanced')}
                                            style={{
                                                flex: 1,
                                                padding: '9px',
                                                borderRadius: '6px',
                                                border: 'none',
                                                cursor: 'pointer',
                                                fontSize: '12px',
                                                fontWeight: 600,
                                                backgroundColor:
                                                    connectionMode === 'advanced' ? 'var(--color-accent, #3a86ff)' : 'transparent',
                                                color: connectionMode === 'advanced' ? '#fff' : 'var(--text-muted, #585866)',
                                            }}
                                        >
                                            Advanced (Full URI / SSL)
                                        </button>
                                    </div>
                                )}

                                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                    <label
                                        style={{
                                            fontSize: '11px',
                                            textTransform: 'uppercase',
                                            fontWeight: 700,
                                            color: 'var(--text-muted, #585866)',
                                            letterSpacing: '0.5px',
                                        }}
                                    >
                                        Connection Name (Optional)
                                    </label>
                                    <input
                                        type="text"
                                        value={name}
                                        onChange={(e) => setName(e.target.value)}
                                        placeholder="e.g. Local Analytics DB"
                                        style={{
                                            backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                            border: '1px solid var(--border-primary, #22222a)',
                                            borderRadius: '8px',
                                            padding: '11px 14px',
                                            fontSize: '13px',
                                            color: 'var(--text-primary, #e6e6eb)',
                                            outline: 'none',
                                        }}
                                    />
                                </div>

                                {isFileDriver ? (
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                        <label
                                            style={{
                                                fontSize: '11px',
                                                textTransform: 'uppercase',
                                                fontWeight: 700,
                                                color: 'var(--text-muted, #585866)',
                                                letterSpacing: '0.5px',
                                            }}
                                        >
                                            {driver === 'sled' ? 'Database Directory Path' : 'Database File Path'}
                                        </label>
                                        <div style={{ display: 'flex', gap: '10px' }}>
                                            <input
                                                type="text"
                                                value={filePath}
                                                onChange={(e) => setFilePath(e.target.value)}
                                                placeholder={
                                                    driver === 'sled'
                                                        ? '/path/to/sled_dir'
                                                        : `/path/to/database.${driver === 'sqlite' ? 'db' : driver}`
                                                }
                                                style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                    border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px',
                                                    padding: '11px 14px',
                                                    fontSize: '13px',
                                                    color: 'var(--text-primary, #e6e6eb)',
                                                    outline: 'none',
                                                    flex: 1,
                                                }}
                                            />
                                            {driver !== 'sled' && (
                                                <button
                                                    type="button"
                                                    onClick={handleBrowseFile}
                                                    style={{
                                                        backgroundColor: 'var(--bg-tertiary, #1b1b22)',
                                                        color: 'var(--text-primary)',
                                                        border: '1px solid var(--border-primary)',
                                                        borderRadius: '8px',
                                                        padding: '0 18px',
                                                        cursor: 'pointer',
                                                        display: 'flex',
                                                        alignItems: 'center',
                                                        gap: '8px',
                                                        fontSize: '13px',
                                                        fontWeight: 500,
                                                    }}
                                                >
                                                    <FolderOpen size={15} /> Browse...
                                                </button>
                                            )}
                                        </div>
                                    </div>
                                ) : connectionMode === 'advanced' ? (
                                    <>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label
                                                style={{
                                                    fontSize: '11px',
                                                    textTransform: 'uppercase',
                                                    fontWeight: 700,
                                                    color: 'var(--text-muted, #585866)',
                                                    letterSpacing: '0.5px',
                                                }}
                                            >
                                                Connection URI
                                            </label>
                                            <textarea
                                                value={customUri}
                                                onChange={(e) => setCustomUri(e.target.value)}
                                                required
                                                placeholder={`e.g. ${driver}://user:pass@host:port/db`}
                                                rows={3}
                                                style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                    border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px',
                                                    padding: '11px 14px',
                                                    fontSize: '13px',
                                                    color: 'var(--text-primary, #e6e6eb)',
                                                    outline: 'none',
                                                    resize: 'vertical',
                                                }}
                                            />
                                        </div>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label
                                                style={{
                                                    fontSize: '11px',
                                                    textTransform: 'uppercase',
                                                    fontWeight: 700,
                                                    color: 'var(--text-muted, #585866)',
                                                    letterSpacing: '0.5px',
                                                }}
                                            >
                                                Authentication Type
                                            </label>
                                            <AlouetteSelect
                                                value={authType}
                                                onChange={setAuthType}
                                                searchable={false}
                                                options={[
                                                    { value: 'basic', label: 'Basic / Password' },
                                                    { value: 'x509_cert', label: 'X.509 Certificate (SSL/TLS)' },
                                                ]}
                                            />
                                        </div>
                                        {authType === 'x509_cert' && (
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                                <label
                                                    style={{
                                                        fontSize: '11px',
                                                        textTransform: 'uppercase',
                                                        fontWeight: 700,
                                                        color: 'var(--text-muted, #585866)',
                                                        letterSpacing: '0.5px',
                                                    }}
                                                >
                                                    SSL Certificate Path (.pem / .crt)
                                                </label>
                                                <div style={{ display: 'flex', gap: '10px' }}>
                                                    <input
                                                        type="text"
                                                        value={sslCert}
                                                        onChange={(e) => setSslCert(e.target.value)}
                                                        required
                                                        placeholder="/path/to/cert.pem"
                                                        style={{
                                                            backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                            border: '1px solid var(--border-primary, #22222a)',
                                                            borderRadius: '8px',
                                                            padding: '11px 14px',
                                                            fontSize: '13px',
                                                            color: 'var(--text-primary, #e6e6eb)',
                                                            outline: 'none',
                                                            flex: 1,
                                                        }}
                                                    />
                                                    <button
                                                        type="button"
                                                        onClick={handleBrowseCert}
                                                        style={{
                                                            backgroundColor: 'var(--bg-tertiary, #1b1b22)',
                                                            color: 'var(--text-primary)',
                                                            border: '1px solid var(--border-primary)',
                                                            borderRadius: '8px',
                                                            padding: '0 18px',
                                                            cursor: 'pointer',
                                                            display: 'flex',
                                                            alignItems: 'center',
                                                            gap: '8px',
                                                            fontSize: '13px',
                                                        }}
                                                    >
                                                        <FolderOpen size={15} /> Browse...
                                                    </button>
                                                </div>
                                            </div>
                                        )}
                                    </>
                                ) : (
                                    <>
                                        <div style={{ display: 'flex', gap: '16px' }}>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 3 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                                                    <Server size={12} /> Host
                                                </label>
                                                <input
                                                    type="text"
                                                    value={host}
                                                    onChange={(e) => setHost(e.target.value)}
                                                    required
                                                    placeholder={driver === 'supabase' ? 'db.xxxx.supabase.co' : 'localhost'}
                                                    style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                        border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px',
                                                        padding: '11px 14px',
                                                        fontSize: '13px',
                                                        color: 'var(--text-primary, #e6e6eb)',
                                                        outline: 'none',
                                                    }}
                                                />
                                            </div>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted)' }}>
                                                    Port
                                                </label>
                                                <input
                                                    type="text"
                                                    value={port}
                                                    onChange={(e) => setPort(e.target.value)}
                                                    required
                                                    style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                        border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px',
                                                        padding: '11px 14px',
                                                        fontSize: '13px',
                                                        color: 'var(--text-primary, #e6e6eb)',
                                                        outline: 'none',
                                                    }}
                                                />
                                            </div>
                                        </div>

                                        <div style={{ display: 'flex', gap: '16px' }}>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                                                    <User size={12} /> Username {driver === 'redis' || driver === 'mongodb' ? '(Optional)' : ''}
                                                </label>
                                                <input
                                                    type="text"
                                                    value={user}
                                                    onChange={(e) => setUser(e.target.value)}
                                                    required={driver !== 'redis' && driver !== 'mongodb'}
                                                    style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                        border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px',
                                                        padding: '11px 14px',
                                                        fontSize: '13px',
                                                        color: 'var(--text-primary, #e6e6eb)',
                                                        outline: 'none',
                                                    }}
                                                />
                                            </div>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                                                    <Lock size={12} /> Password
                                                </label>
                                                <input
                                                    type="password"
                                                    value={password}
                                                    onChange={(e) => setPassword(e.target.value)}
                                                    required={driver === 'supabase'}
                                                    style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                        border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px',
                                                        padding: '11px 14px',
                                                        fontSize: '13px',
                                                        color: 'var(--text-primary, #e6e6eb)',
                                                        outline: 'none',
                                                    }}
                                                />
                                            </div>
                                        </div>

                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: '6px' }}>
                                                <Database size={12} />{' '}
                                                {driver === 'redis' ? 'Database Index (Optional, e.g. 0)' : 'Database Name'}
                                            </label>
                                            <input
                                                type="text"
                                                value={database}
                                                onChange={(e) => setDatabase(e.target.value)}
                                                required={driver !== 'redis' && driver !== 'mongodb'}
                                                style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)',
                                                    border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px',
                                                    padding: '11px 14px',
                                                    fontSize: '13px',
                                                    color: 'var(--text-primary, #e6e6eb)',
                                                    outline: 'none',
                                                }}
                                            />
                                        </div>
                                    </>
                                )}

                                {!isFileDriver && (
                                    <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                                        <input
                                            type="checkbox"
                                            checked={rememberCredentials}
                                            onChange={(e) => setRememberCredentials(e.target.checked)}
                                        />
                                        Save full connection URI locally for 1-click reconnect (uncheck to mask password in Saved Connections)
                                    </label>
                                )}

                                <div style={{ marginTop: '8px', paddingTop: '20px', borderTop: '1px solid var(--border-primary, #22222a)' }}>
                                    <button
                                        form="db-conn-form"
                                        type="submit"
                                        disabled={loading}
                                        style={{
                                            backgroundColor: 'var(--color-accent, #3a86ff)',
                                            color: '#fff',
                                            border: 'none',
                                            borderRadius: '8px',
                                            padding: '13px 24px',
                                            fontSize: '14px',
                                            fontWeight: 600,
                                            cursor: loading ? 'not-allowed' : 'pointer',
                                            opacity: loading ? 0.7 : 1,
                                            display: 'flex',
                                            alignItems: 'center',
                                            justifyContent: 'center',
                                            width: '100%',
                                        }}
                                    >
                                        {loading ? 'Connecting...' : `Connect to ${driver.toUpperCase()}`}
                                    </button>
                                </div>
                            </form>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
