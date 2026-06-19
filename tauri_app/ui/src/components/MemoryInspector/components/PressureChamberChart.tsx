import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { TelemetryData } from '../types';

interface ChartProps {
    data: TelemetryData[];
}

export function PressureChamberChart({ data }: ChartProps) {
    const chartData = data.map(d => ({
        time: new Date(d.timestamp * 1000).toLocaleTimeString(),
        usage: d.memory_usage_mb,
        limit: d.memory_limit_mb
    }));

    return (
        <div className="inspector-chart-container">
            <h3 className="chart-title">Pressure Chamber</h3>
            <div className="chart-wrapper">
                <ResponsiveContainer width="100%" height={300}>
                    <AreaChart data={chartData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                        <defs>
                            <linearGradient id="colorUsage" x1="0" y1="0" x2="0" y2="1">
                                <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.8}/>
                                <stop offset="95%" stopColor="#3b82f6" stopOpacity={0}/>
                            </linearGradient>
                            <linearGradient id="colorLimit" x1="0" y1="0" x2="0" y2="1">
                                <stop offset="5%" stopColor="#ef4444" stopOpacity={0.3}/>
                                <stop offset="95%" stopColor="#ef4444" stopOpacity={0}/>
                            </linearGradient>
                        </defs>
                        <XAxis dataKey="time" stroke="#6b7280" fontSize={12} />
                        <YAxis stroke="#6b7280" fontSize={12} />
                        <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
                        <Tooltip 
                            contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                            itemStyle={{ color: '#e5e7eb' }}
                        />
                        <Area 
                            type="monotone" 
                            dataKey="limit" 
                            stroke="#ef4444" 
                            strokeWidth={2}
                            fillOpacity={1} 
                            fill="url(#colorLimit)" 
                            name="Memory Limit"
                        />
                        <Area 
                            type="monotone" 
                            dataKey="usage" 
                            stroke="#3b82f6" 
                            strokeWidth={2}
                            fillOpacity={1} 
                            fill="url(#colorUsage)" 
                            name="Memory Usage"
                        />
                    </AreaChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
}
