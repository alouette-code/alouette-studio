import 'dart:math';
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../models/project.dart';
import '../theme/app_theme.dart';

/// Project resource monitoring panel mirroring ProjectResources.tsx.
/// Shows CPU/RAM/GPU stat cards with progress bars, uptime counter,
/// project meta info, and resource sparkline history charts.
class ProjectResourcesWidget extends StatefulWidget {
  final Project? activeProject;
  final ProcessState activeState;
  final ResourceHistory resourceHistory;

  const ProjectResourcesWidget({
    super.key,
    this.activeProject,
    required this.activeState,
    required this.resourceHistory,
  });

  @override
  State<ProjectResourcesWidget> createState() =>
      _ProjectResourcesWidgetState();
}

class _ProjectResourcesWidgetState extends State<ProjectResourcesWidget> {
  int _localUptime = 0;
  Timer? _uptimeTimer;

  @override
  void dispose() {
    _uptimeTimer?.cancel();
    super.dispose();
  }

  void _startUptimeTimer(ProcessStateType? type) {
    if (type == ProcessStateType.running ||
        type == ProcessStateType.setup) {
      _uptimeTimer ??=
          Timer.periodic(const Duration(seconds: 1), (_) {
        if (mounted) {
          setState(() => _localUptime++);
        }
      });
    } else {
      _uptimeTimer?.cancel();
      _uptimeTimer = null;
      if (mounted) {
        setState(() => _localUptime = 0);
      }
    }
  }

  String _formatUptime(int totalSeconds) {
    final hrs = totalSeconds ~/ 3600;
    final mins = (totalSeconds % 3600) ~/ 60;
    final secs = totalSeconds % 60;
    return '${hrs.toString().padLeft(2, '0')}:${mins.toString().padLeft(2, '0')}:${secs.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final project = provider.activeProject;
        final state = provider.activeState;
        final resources = provider.resourceHistory;
        final isRunning = state.type == ProcessStateType.running;
        final isSetup = state.type == ProcessStateType.setup;

        // Manage uptime timer
        _startUptimeTimer(state.type);

        if (project == null) {
          return _buildEmptyState(context);
        }

        final history = resources.projects[project.id];
        final cpuHistory = history?.cpu ?? [];
        final ramHistory = history?.ram ?? [];

        final currentCpu =
            cpuHistory.isNotEmpty ? cpuHistory.last : 0.0;
        final currentRam =
            ramHistory.isNotEmpty ? ramHistory.last : 0.0;

        // Simulated GPU reading (mirrors React: sin-based mock)
        final currentGpu = isRunning
            ? ((sin(_localUptime / 5.0) * 2 + 3).floorToDouble())
            : 0.0;

        // Extract PID from state data
        final pidVal = isRunning
            ? (state.data is Map ? state.data['pid'] : state.data)
            : null;

        final maxCpu = (project.maxCpuPercent ?? 20.0).toDouble();
        final maxRam = (project.maxRamMb ?? 2000).toDouble();

        return SingleChildScrollView(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Main Grid: Statistics cards
              _buildStatsGrid(context, currentCpu, currentRam,
                  currentGpu.toDouble(), maxCpu, maxRam, isRunning, pidVal),

              const SizedBox(height: 16),

              // Project Meta Information
              _buildMetaSection(context, project, state),
            ],
          ),
        );
      },
    );
  }

  Widget _buildEmptyState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.monitor_heart_outlined,
              size: 40,
              color: AppTheme.textSecondary(context).withOpacity(0.4)),
          const SizedBox(height: 12),
          Text('No Project Selected',
              style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 14,
                  fontWeight: FontWeight.w600)),
          const SizedBox(height: 4),
          Text(
              'Select a project from the explorer or tab list to view its resource statistics.',
              textAlign: TextAlign.center,
              style: TextStyle(
                  color: AppTheme.textSecondary(context), fontSize: 11)),
        ],
      ),
    );
  }

  Widget _buildStatsGrid(BuildContext context, double cpu, double ram,
      double gpu, double maxCpu, double maxRam, bool isRunning, dynamic pid) {
    return Column(
      children: [
        // 2x2 Grid
        Row(
          children: [
            Expanded(
              child: _StatCard(
                title: 'CPU',
                value: cpu.toStringAsFixed(1),
                unit: '%',
                progress: (cpu / 100.0).clamp(0.0, 1.0),
                progressColor: AppColors.darkInfo,
                limitLabel:
                    'Limit: ${maxCpu.toStringAsFixed(0)}%',
                metaLabel: 'Cores: Auto-allocated',
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: _StatCard(
                title: 'RAM',
                value: ram.toStringAsFixed(1),
                unit: 'MB',
                progress: (ram / maxRam).clamp(0.0, 1.0),
                progressColor: AppColors.darkAccentPurple,
                limitLabel:
                    'Limit: ${maxRam.toStringAsFixed(0)} MB',
                metaLabel:
                    'Load: ${((ram / maxRam) * 100).toStringAsFixed(0)}%',
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            Expanded(
              child: _StatCard(
                title: 'GPU',
                value: isRunning ? gpu.toStringAsFixed(0) : '0',
                unit: '%',
                progress: isRunning ? (gpu / 100.0).clamp(0.0, 1.0) : 0.0,
                progressColor: const Color(0xFF8B5CF6),
                limitLabel: 'Engine: Direct3D12 / Vulkan',
                metaLabel:
                    'Status: ${isRunning ? "Active" : "Inactive"}',
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: _UptimeCard(
                uptime: _formatUptime(_localUptime),
                isRunning: isRunning,
                statusType: isRunning ? 'Running' : 'Stopped',
                pid: pid,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildMetaSection(
      BuildContext context, Project project, ProcessState state) {
    final isRunning = state.type == ProcessStateType.running;
    final pidVal = isRunning
        ? (state.data is Map ? state.data['pid'] : state.data)
        : null;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(Icons.info_outline,
                size: 12, color: AppTheme.textSecondary(context)),
            const SizedBox(width: 4),
            Text('Project Details',
                style: TextStyle(
                    color: AppTheme.textPrimary(context),
                    fontSize: 11,
                    fontWeight: FontWeight.w600)),
          ],
        ),
        const SizedBox(height: 8),
        Container(
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(
            color: Theme.of(context).brightness == Brightness.dark
                ? AppColors.darkSurface
                : AppColors.lightSurface,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: AppTheme.borderPrimary(context)),
          ),
          child: Column(
            children: [
              _MetaInfoRow(
                label: 'Port',
                value: project.port?.toString() ?? 'Not set',
                valueColor: AppColors.darkSuccess,
              ),
              const Divider(
                  height: 12,
                  color: AppColors.darkBorder),
              _MetaInfoRow(
                label: 'CWD',
                value: project.cwd ?? 'Default (Root)',
              ),
              const Divider(
                  height: 12,
                  color: AppColors.darkBorder),
              _MetaInfoRow(
                label: 'Command',
                value:
                    '${project.command} ${project.args.join(' ')}',
                mono: true,
              ),
              const Divider(
                  height: 12,
                  color: AppColors.darkBorder),
              _MetaInfoRow(
                label: 'Security',
                value: project.enableTunnel
                    ? 'Cloudflare Tunnel Enabled'
                    : 'Standard Sandbox',
              ),
              const Divider(
                  height: 12,
                  color: AppColors.darkBorder),
              _MetaInfoRow(
                label: 'PID',
                value: pidVal?.toString() ?? 'N/A',
                mono: true,
              ),
              const Divider(
                  height: 12,
                  color: AppColors.darkBorder),
              _MetaInfoRow(
                label: 'Status',
                value: state.type.label,
                valueColor: isRunning
                    ? AppColors.darkSuccess
                    : AppColors.darkTextSecondary,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// Stat card with title, value, progress bar, and meta info.
class _StatCard extends StatelessWidget {
  final String title;
  final String value;
  final String unit;
  final double progress;
  final Color progressColor;
  final String limitLabel;
  final String metaLabel;

  const _StatCard({
    required this.title,
    required this.value,
    required this.unit,
    required this.progress,
    required this.progressColor,
    required this.limitLabel,
    required this.metaLabel,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: Theme.of(context).brightness == Brightness.dark
            ? AppColors.darkSurface
            : AppColors.lightSurface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppTheme.borderPrimary(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Title
          Text(title,
              style: TextStyle(
                  color: AppTheme.textSecondary(context),
                  fontSize: 10,
                  fontWeight: FontWeight.w600)),
          const SizedBox(height: 6),
          // Value display
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Text(value,
                  style: TextStyle(
                      color: AppTheme.textPrimary(context),
                      fontSize: 22,
                      fontWeight: FontWeight.w700,
                      fontFamily: 'monospace')),
              const SizedBox(width: 3),
              Padding(
                padding: const EdgeInsets.only(bottom: 3),
                child: Text(unit,
                    style: TextStyle(
                        color: AppTheme.textSecondary(context),
                        fontSize: 11)),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Progress bar
          ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: LinearProgressIndicator(
              value: progress.clamp(0.0, 1.0),
              backgroundColor:
                  AppTheme.borderPrimary(context),
              valueColor: AlwaysStoppedAnimation<Color>(
                progress > 0.8
                    ? AppColors.darkDanger
                    : progress > 0.5
                        ? AppColors.darkWarning
                        : progressColor,
              ),
              minHeight: 5,
            ),
          ),
          const SizedBox(height: 6),
          // Meta info
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(limitLabel,
                  style: TextStyle(
                      color: AppTheme.textSecondary(context)
                          .withOpacity(0.7),
                      fontSize: 9)),
              Text(metaLabel,
                  style: TextStyle(
                      color: AppTheme.textSecondary(context)
                          .withOpacity(0.7),
                      fontSize: 9)),
            ],
          ),
        ],
      ),
    );
  }
}

/// Uptime card showing formatted uptime and status.
class _UptimeCard extends StatelessWidget {
  final String uptime;
  final bool isRunning;
  final String statusType;
  final dynamic pid;

  const _UptimeCard({
    required this.uptime,
    required this.isRunning,
    required this.statusType,
    required this.pid,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: Theme.of(context).brightness == Brightness.dark
            ? AppColors.darkSurface
            : AppColors.lightSurface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppTheme.borderPrimary(context)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Uptime',
              style: TextStyle(
                  color: AppTheme.textSecondary(context),
                  fontSize: 10,
                  fontWeight: FontWeight.w600)),
          const SizedBox(height: 6),
          Text(uptime,
              style: TextStyle(
                  color: AppColors.darkSuccess,
                  fontSize: 20,
                  fontWeight: FontWeight.w700,
                  fontFamily: 'monospace')),
          const SizedBox(height: 4),
          Text(
            isRunning
                ? 'Project running continuously'
                : 'Project is stopped',
            style: TextStyle(
                color: AppTheme.textSecondary(context),
                fontSize: 9),
          ),
          const SizedBox(height: 6),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Status: $statusType',
                  style: TextStyle(
                      color: isRunning
                          ? AppColors.darkSuccess
                          : AppTheme.textSecondary(context)
                              .withOpacity(0.7),
                      fontSize: 9,
                      fontWeight:
                          isRunning ? FontWeight.w600 : FontWeight.normal)),
              Text('PID: ${pid ?? "N/A"}',
                  style: TextStyle(
                      color: AppTheme.textSecondary(context)
                          .withOpacity(0.7),
                      fontSize: 9,
                      fontFamily: 'monospace')),
            ],
          ),
        ],
      ),
    );
  }
}

/// Meta info row for the project details section.
class _MetaInfoRow extends StatelessWidget {
  final String label;
  final String value;
  final bool mono;
  final Color? valueColor;

  const _MetaInfoRow({
    required this.label,
    required this.value,
    this.mono = false,
    this.valueColor,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 60,
          child: Text(label,
              style: TextStyle(
                  color: AppTheme.textSecondary(context),
                  fontSize: 10,
                  fontWeight: FontWeight.w600)),
        ),
        Expanded(
          child: Tooltip(
            message: value,
            child: Text(
              value,
              style: TextStyle(
                color: valueColor ??
                    AppTheme.textPrimary(context),
                fontSize: 10,
                fontFamily: mono ? 'monospace' : null,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
      ],
    );
  }
}
