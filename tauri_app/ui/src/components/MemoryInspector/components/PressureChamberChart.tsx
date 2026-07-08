import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, LabelList, Legend } from 'recharts';
import { TelemetryData } from '../types';

interface ChartProps {
    data: TelemetryData[];
}

export function PressureChamberChart({ data }: ChartProps) {
    const chartData = data.map(d => ({
        time: new Date(d.timestamp * 1000).toLocaleTimeString(),
        usage: Number(d.memory_usage_mb.toFixed(1)),
        limit: Number(d.memory_limit_mb.toFixed(1))
    }));

    return (
        <div className="inspector-chart-container">
            <h3 className="panel-title">Pressure Chamber</h3>
            <div className="chart-wrapper">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={chartData} margin={{ top: 20, right: 30, left: 0, bottom: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="#22222a" vertical={false} />
                        <XAxis dataKey="time" stroke="#92929e" fontSize={11} tickMargin={10} />
                        <YAxis stroke="#92929e" fontSize={11} />
                        <Tooltip 
                            contentStyle={{ backgroundColor: '#111115', border: '1px solid #22222a' }}
                            itemStyle={{ color: '#e6e6eb' }}
                            cursor={{ fill: 'rgba(255,255,255,0.05)' }}
                        />
                        <Legend wrapperStyle={{ fontSize: '12px', color: '#92929e' }} />
                        <Bar dataKey="limit" name="Memory Limit (MB)" fill="#ef4444" radius={[2, 2, 0, 0]} opacity={0.3} />
                        <Bar dataKey="usage" name="Memory Usage (MB)" fill="#3a86ff" radius={[2, 2, 0, 0]}>
                            <LabelList dataKey="usage" position="top" fill="#e6e6eb" fontSize={10} />
                        </Bar>
                    </BarChart>
                </ResponsiveContainer>
            </div>
        </div>
    );
}
