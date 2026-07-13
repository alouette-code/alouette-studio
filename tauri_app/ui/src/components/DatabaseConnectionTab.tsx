import React, { useState } from 'react';
import { Database, Lock, Server, User, Cable, ArrowLeft, FolderOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export interface DbConnection {
    id: string;
    name: string;
    uri: string;
    driver: 'mysql' | 'postgres' | 'sqlite' | 'mongodb' | 'redis' | 'supabase' | 'firebase' | 'mssql';
}

interface Props {
    onConnect: (uri: string) => void;
}

export default function DatabaseConnectionTab({ onConnect }: Props) {
    const [step, setStep] = useState<1 | 2>(1);
    const [driver, setDriver] = useState<'mysql' | 'postgres' | 'sqlite' | 'mongodb' | 'redis' | 'supabase' | 'firebase' | 'mssql'>('postgres');
    
    // Connection form state
    const [name, setName] = useState('');
    const [host, setHost] = useState('localhost');
    const [port, setPort] = useState('5432');
    const [user, setUser] = useState('postgres');
    const [password, setPassword] = useState('');
    const [database, setDatabase] = useState('');
    const [sqlitePath, setSqlitePath] = useState('');
    
    // Advanced / Cloud Mode state
    const [connectionMode, setConnectionMode] = useState<'standard' | 'advanced'>('standard');
    const [customUri, setCustomUri] = useState('');
    const [authType, setAuthType] = useState('basic');
    const [sslCert, setSslCert] = useState('');
    
    // Firebase specific
    const [projectId, setProjectId] = useState('');
    
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleSelectDriver = (selected: 'mysql' | 'postgres' | 'sqlite' | 'mongodb' | 'redis' | 'supabase' | 'firebase' | 'mssql') => {
        setDriver(selected);
        if (selected === 'mysql') setPort('3306');
        if (selected === 'postgres' || selected === 'supabase') setPort('5432');
        if (selected === 'mongodb') setPort('27017');
        if (selected === 'redis') setPort('6379');
        if (selected === 'mssql') setPort('1433');
        setError(null);
        setStep(2);
    };

    const handleBrowseSqlite = async () => {
        try {
            const selectedPath: string | null = await invoke("open_file_dialog");
            if (selectedPath) {
                setSqlitePath(selectedPath);
            }
        } catch (err: any) {
            setError(err.toString());
        }
    };

    const handleBrowseCert = async () => {
        try {
            const selectedPath: string | null = await invoke("open_file_dialog");
            if (selectedPath) {
                setSslCert(selectedPath);
            }
        } catch (err: any) {
            setError(err.toString());
        }
    };

    const handleConnect = async (e?: React.FormEvent) => {
        if (e) e.preventDefault();
        setLoading(true);
        setError(null);
        
        let uri = '';
        if (driver !== 'sqlite' && driver !== 'firebase' && connectionMode === 'advanced') {
            if (!customUri) {
                setError('Connection URI is required in Advanced mode.');
                setLoading(false);
                return;
            }
            uri = customUri;
        } else if (driver === 'sqlite') {
            if (!sqlitePath) {
                setError('Please select a SQLite database file.');
                setLoading(false);
                return;
            }
            uri = `sqlite://${sqlitePath}`;
        } else if (driver === 'firebase') {
            if (!projectId || !password) {
                setError('Project ID and API Key are required for Firebase.');
                setLoading(false);
                return;
            }
            uri = `firebase://${projectId}:${password}@firebase`;
        } else if (driver === 'redis') {
            const auth = password ? `:${password}@` : '';
            const dbPath = database ? `/${database}` : '';
            uri = `redis://${auth}${host}:${port}${dbPath}`;
        } else if (driver === 'mongodb') {
            const auth = (user && password) ? `${user}:${password}@` : '';
            const dbPath = database ? `/${database}` : '';
            uri = `mongodb://${auth}${host}:${port}${dbPath}`;
        } else if (driver === 'mssql') {
            const auth = (user && password) ? `${user}:${password}@` : '';
            const dbPath = database ? `/${database}` : '';
            uri = `mssql://${auth}${host}:${port}${dbPath}`;
        } else if (driver === 'supabase') {
            // Supabase is basically Postgres
            uri = `postgres://${user}:${password}@${host}:${port}/${database}`;
        } else {
            if (!database) {
                setError('Database name is required.');
                setLoading(false);
                return;
            }
            uri = `${driver}://${user}:${password}@${host}:${port}/${database}`;
        }
        
        try {
            let res = { success: true, message: '' };
            if (driver !== 'sqlite') {
                res = await invoke<{ success: boolean; message: string }>('connect_to_db', { 
                    options: { 
                        uri, 
                        auth_type: (connectionMode === 'advanced') ? authType : 'basic',
                        ssl_cert: (connectionMode === 'advanced' && authType === 'x509_cert') ? sslCert : null
                    } 
                });
            }
            
            if (res.success) {
                const newConn: DbConnection = {
                    id: crypto.randomUUID(),
                    name: name || (driver === 'sqlite' ? sqlitePath.split(/[/\\]/).pop()! : database || projectId) || 'New Connection',
                    uri,
                    driver
                };
                
                const existing = JSON.parse(localStorage.getItem('db_connections') || '[]');
                localStorage.setItem('db_connections', JSON.stringify([...existing, newConn]));
                
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
        <div style={{
            width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
            backgroundColor: 'var(--bg-primary, #0a0a0c)', color: 'var(--text-primary, #e6e6eb)',
            overflowY: 'auto', padding: '40px'
        }}>
            <div style={{ maxWidth: '800px', width: '100%', margin: '0 auto' }}>
                
                {/* Header */}
                <div style={{ marginBottom: '32px' }}>
                    <h1 style={{ fontSize: '24px', fontWeight: 700, margin: '0 0 8px 0', display: 'flex', alignItems: 'center', gap: '12px' }}>
                        <Cable size={28} color="var(--color-accent, #3a86ff)" />
                        Universal Database Setup
                    </h1>
                    <p style={{ color: 'var(--text-muted, #585866)', margin: 0, fontSize: '13px' }}>
                        Configure a new connection to SQL or NoSQL database instances.
                    </p>
                </div>

                {/* Step 1: Select Driver */}
                {step === 1 && (
                    <div className="animate-in fade-in slide-in-from-bottom-4 duration-300">
                        <h2 style={{ fontSize: '14px', fontWeight: 600, marginBottom: '16px', color: 'var(--text-secondary, #92929e)' }}>
                            Step 1: Select Database Engine
                        </h2>
                        
                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))', gap: '16px' }}>
                            {/* PostgreSQL Card */}
                            <div onClick={() => handleSelectDriver('postgres')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>PostgreSQL</span>
                            </div>

                            {/* MySQL Card */}
                            <div onClick={() => handleSelectDriver('mysql')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>MySQL</span>
                            </div>

                            {/* SQLite Card */}
                            <div onClick={() => handleSelectDriver('sqlite')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>SQLite</span>
                            </div>

                            {/* MongoDB Card */}
                            <div onClick={() => handleSelectDriver('mongodb')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>MongoDB</span>
                            </div>

                            {/* Redis Card */}
                            <div onClick={() => handleSelectDriver('redis')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>Redis</span>
                            </div>
                            
                            {/* Supabase Card */}
                            <div onClick={() => handleSelectDriver('supabase')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>Supabase</span>
                            </div>

                            {/* Firebase Card */}
                            <div onClick={() => handleSelectDriver('firebase')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>Firebase</span>
                            </div>

                            {/* SQL Server Card */}
                            <div onClick={() => handleSelectDriver('mssql')} style={{
                                backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)', borderRadius: '12px', padding: '24px', cursor: 'pointer', transition: 'color 0.2s', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '12px', color: 'var(--text-secondary, #92929e)'
                            }} onMouseEnter={(e) => e.currentTarget.style.color = '#ffffff'} onMouseLeave={(e) => e.currentTarget.style.color = 'var(--text-secondary, #92929e)'}>
                                <Server size={48} color="currentColor" />
                                <span style={{ fontWeight: 600, fontSize: '14px' }}>SQL Server</span>
                            </div>
                        </div>
                    </div>
                )}

                {/* Step 2: Configure Connection */}
                {step === 2 && (
                    <div className="animate-in fade-in slide-in-from-right-8 duration-300">
                        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '24px' }}>
                            <button onClick={() => setStep(1)} style={{
                                background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer',
                                display: 'flex', alignItems: 'center', gap: '6px', padding: '6px 12px', borderRadius: '6px',
                                backgroundColor: 'rgba(255,255,255,0.05)'
                            }}>
                                <ArrowLeft size={14} /> Back
                            </button>
                            <h2 style={{ fontSize: '14px', fontWeight: 600, margin: 0, color: 'var(--text-secondary, #92929e)' }}>
                                Step 2: Configure {driver.toUpperCase()}
                            </h2>
                        </div>

                        <div style={{
                            backgroundColor: 'var(--bg-secondary, #111115)', border: '1px solid var(--border-primary, #22222a)',
                            borderRadius: '12px', padding: '32px',
                        }}>
                            <form id="db-conn-form" onSubmit={handleConnect} style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                                {error && (
                                    <div style={{
                                        padding: '12px 16px', backgroundColor: 'rgba(239, 68, 68, 0.1)',
                                        border: '1px solid rgba(239, 68, 68, 0.2)', borderRadius: '8px',
                                        color: 'var(--color-danger, #ef4444)', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '8px'
                                    }}>
                                        {error}
                                    </div>
                                )}
                                
                                {driver !== 'sqlite' && driver !== 'firebase' && (
                                    <div style={{ display: 'flex', gap: '8px', marginBottom: '8px', backgroundColor: 'var(--bg-primary, #0a0a0c)', padding: '6px', borderRadius: '8px' }}>
                                        <button type="button" onClick={() => setConnectionMode('standard')} style={{
                                            flex: 1, padding: '10px', borderRadius: '6px', border: 'none', cursor: 'pointer', fontSize: '13px', fontWeight: 600,
                                            backgroundColor: connectionMode === 'standard' ? 'var(--color-accent, #3a86ff)' : 'transparent',
                                            color: connectionMode === 'standard' ? '#fff' : 'var(--text-muted, #585866)',
                                            transition: 'all 0.2s'
                                        }}>Standard (Local)</button>
                                        <button type="button" onClick={() => setConnectionMode('advanced')} style={{
                                            flex: 1, padding: '10px', borderRadius: '6px', border: 'none', cursor: 'pointer', fontSize: '13px', fontWeight: 600,
                                            backgroundColor: connectionMode === 'advanced' ? 'var(--color-accent, #3a86ff)' : 'transparent',
                                            color: connectionMode === 'advanced' ? '#fff' : 'var(--text-muted, #585866)',
                                            transition: 'all 0.2s'
                                        }}>Advanced (Cloud URI)</button>
                                    </div>
                                )}
                                
                                {/* Connection Name (Common) */}
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                    <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                        Connection Name (Optional)
                                    </label>
                                    <input type="text" value={name} onChange={e => setName(e.target.value)} placeholder="e.g. My Database" style={{
                                        backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                        borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                    }} />
                                </div>

                                {driver === 'sqlite' ? (
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                        <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                            Database File Path
                                        </label>
                                        <div style={{ display: 'flex', gap: '12px' }}>
                                            <input type="text" value={sqlitePath} onChange={e => setSqlitePath(e.target.value)} placeholder="/path/to/database.db" style={{
                                                backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none', flex: 1
                                            }} />
                                            <button type="button" onClick={handleBrowseSqlite} style={{
                                                backgroundColor: 'var(--bg-tertiary, #1b1b22)', color: 'var(--text-primary)', border: '1px solid var(--border-primary)',
                                                borderRadius: '8px', padding: '0 20px', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', fontWeight: 500
                                            }}>
                                                <FolderOpen size={16} /> Browse...
                                            </button>
                                        </div>
                                    </div>
                                ) : driver === 'firebase' ? (
                                    <>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                                Project ID
                                            </label>
                                            <input type="text" value={projectId} onChange={e => setProjectId(e.target.value)} required placeholder="e.g. my-firebase-project" style={{
                                                backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                            }} />
                                        </div>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                                Web API Key
                                            </label>
                                            <input type="password" value={password} onChange={e => setPassword(e.target.value)} required placeholder="AIzaSy..." style={{
                                                backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                            }} />
                                        </div>
                                    </>
                                ) : connectionMode === 'advanced' ? (
                                    <>
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                                Connection URI
                                            </label>
                                            <textarea value={customUri} onChange={e => setCustomUri(e.target.value)} required placeholder={`e.g. ${driver}://user:pass@host:port/db`} rows={3} style={{
                                                backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none', resize: 'vertical'
                                            }} />
                                        </div>
                                        <div style={{ display: 'flex', gap: '24px' }}>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                                    Authentication Type
                                                </label>
                                                <select value={authType} onChange={e => setAuthType(e.target.value)} style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none'
                                                }}>
                                                    <option value="basic">Basic / Password</option>
                                                    <option value="x509_cert">X.509 Certificate (SSL/TLS)</option>
                                                </select>
                                            </div>
                                        </div>
                                        {authType === 'x509_cert' && (
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '8px' }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>
                                                    SSL Certificate Path (.pem / .crt)
                                                </label>
                                                <div style={{ display: 'flex', gap: '12px' }}>
                                                    <input type="text" value={sslCert} onChange={e => setSslCert(e.target.value)} required placeholder="/path/to/cert.pem" style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none', flex: 1
                                                    }} />
                                                    <button type="button" onClick={handleBrowseCert} style={{
                                                        backgroundColor: 'var(--bg-tertiary, #1b1b22)', color: 'var(--text-primary)', border: '1px solid var(--border-primary)',
                                                        borderRadius: '8px', padding: '0 20px', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px', fontWeight: 500
                                                    }}>
                                                        <FolderOpen size={16} /> Browse...
                                                    </button>
                                                </div>
                                            </div>
                                        )}
                                    </>
                                ) : (
                                    <>
                                        <div style={{ display: 'flex', gap: '24px' }}>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 3 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px', display: 'flex', alignItems: 'center', gap: '6px' }}><Server size={12} /> Host</label>
                                                <input type="text" value={host} onChange={e => setHost(e.target.value)} required placeholder={driver === 'supabase' ? "db.xxxx.supabase.co" : "localhost"} style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                                }} />
                                            </div>
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px' }}>Port</label>
                                                <input type="text" value={port} onChange={e => setPort(e.target.value)} required style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                                }} />
                                            </div>
                                        </div>

                                        <div style={{ display: 'flex', gap: '24px' }}>
                                            {driver !== 'redis' && (
                                                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                    <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px', display: 'flex', alignItems: 'center', gap: '6px' }}><User size={12} /> Username</label>
                                                    <input type="text" value={user} onChange={e => setUser(e.target.value)} required={driver !== 'mongodb'} style={{
                                                        backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                        borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                                    }} />
                                                </div>
                                            )}
                                            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', flex: 1 }}>
                                                <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px', display: 'flex', alignItems: 'center', gap: '6px' }}><Lock size={12} /> Password</label>
                                                <input type="password" value={password} onChange={e => setPassword(e.target.value)} required={driver === 'supabase' || driver === 'mssql'} style={{
                                                    backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                    borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                                }} />
                                            </div>
                                        </div>

                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                            <label style={{ fontSize: '11px', textTransform: 'uppercase', fontWeight: 700, color: 'var(--text-muted, #585866)', letterSpacing: '0.5px', display: 'flex', alignItems: 'center', gap: '6px' }}><Database size={12} /> {driver === 'redis' ? 'Database Index (Optional)' : 'Database Name'}</label>
                                            <input type="text" value={database} onChange={e => setDatabase(e.target.value)} required={driver !== 'redis' && driver !== 'mongodb'} style={{
                                                backgroundColor: 'var(--bg-primary, #0a0a0c)', border: '1px solid var(--border-primary, #22222a)',
                                                borderRadius: '8px', padding: '12px 16px', fontSize: '13px', color: 'var(--text-primary, #e6e6eb)', outline: 'none',
                                            }} />
                                        </div>
                                    </>
                                )}

                                <div style={{ marginTop: '16px', paddingTop: '24px', borderTop: '1px solid var(--border-primary, #22222a)' }}>
                                    <button form="db-conn-form" type="submit" disabled={loading} style={{
                                        backgroundColor: 'var(--color-accent, #3a86ff)', color: '#fff', border: 'none', borderRadius: '8px',
                                        padding: '14px 24px', fontSize: '14px', fontWeight: 600, cursor: loading ? 'not-allowed' : 'pointer',
                                        opacity: loading ? 0.7 : 1, transition: 'background-color 0.2s', display: 'flex', alignItems: 'center',
                                        justifyContent: 'center', width: '100%'
                                    }}>
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
