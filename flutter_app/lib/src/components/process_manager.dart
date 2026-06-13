import 'dart:math';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';
import '../models/project.dart';

/// Process manager panel mirroring ProcessManager.tsx.
/// Shows a tree table of all registered projects with process info,
/// expandable child processes, start/stop/force-kill controls,
/// and deep-dive details (ports, threads, loaded modules).
class ProcessManagerWidget extends StatefulWidget {
  const ProcessManagerWidget({super.key});

  @override
  State<ProcessManagerWidget> createState() => _ProcessManagerWidgetState();
}

class _ProcessManagerWidgetState extends State<ProcessManagerWidget> {
  // Track expanded state for projects (to show child processes)
  final Set<String> _expandedProjects = {};
  // Track expanded state for individual processes (to show ports, threads, maps)
  final Set<int> _expandedProcesses = {};

  void _toggleProjectExpand(String projectId) {
    setState(() {
      if (_expandedProjects.contains(projectId)) {
        _expandedProjects.remove(projectId);
      } else {
        _expandedProjects.add(projectId);
      }
    });
  }

  void _toggleProcessExpand(int pid) {
    setState(() {
      if (_expandedProcesses.contains(pid)) {
        _expandedProcesses.remove(pid);
      } else {
        _expandedProcesses.add(pid);
      }
    });
  }

  String _formatBytes(int bytes) {
    if (bytes == 0) return '0.0 MB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final projects = provider.projects;
        return Container(
          color: Theme.of(context).brightness == Brightness.dark
              ? const Color(0xFF14141E)
              : const Color(0xFFFAFAFC),
          child: Column(
            children: [
              // Header
              _buildHeader(context, projects.length),
              // Table body (scrollable)
              Expanded(
                child: projects.isEmpty
                    ? _buildEmptyState(context)
                    : ListView(
                        children: projects.map((p) {
                          final state =
                              provider.projectStates[p.id];
                          final history =
                              provider.resourceHistory.projects[p.id];
                          final cpu = (history?.cpu.isNotEmpty == true)
                              ? history!.cpu.last
                              : 0.0;
                          final ram = (history?.ram.isNotEmpty == true)
                              ? history!.ram.last
                              : 0.0;
                          final isProjectActive =
                              p.id == provider.activeProjectId;
                          final childProcesses =
                              history?.processes ?? [];
                          final isExpanded =
                              _expandedProjects.contains(p.id);
                          final isRunning = state?.type ==
                                  ProcessStateType.running ||
                              childProcesses.isNotEmpty;

                          return Column(
                            children: [
                              // Project main row
                              _ProjectRow(
                                project: p,
                                state: state,
                                isActive: isProjectActive,
                                isRunning: isRunning,
                                isExpanded: isExpanded,
                                cpu: cpu,
                                ram: ram,
                                hasChildren: isRunning,
                                onSelect: () =>
                                    provider.activeProjectId = p.id,
                                onToggleExpand: () =>
                                    _toggleProjectExpand(p.id),
                                onStart: () =>
                                    provider.handleStartProject(p.id),
                                onStop: () =>
                                    provider.handleStopProject(p.id),
                                onForceKill: () =>
                                    provider.handleStopProject(p.id),
                              ),

                              // Child processes (nested tree)
                              if (isRunning && isExpanded) ...[
                                if (childProcesses.isEmpty)
                                  _buildEmptyProcessRow(context)
                                else
                                  ...childProcesses.map((cp) {
                                    final isProcessExpanded =
                                        _expandedProcesses
                                            .contains(cp.pid);
                                    return _ChildProcessRow(
                                      process: cp,
                                      isExpanded: isProcessExpanded,
                                      onToggleExpand: () =>
                                          _toggleProcessExpand(
                                              cp.pid),
                                      onKill: () {
                                        provider.showConfirm(
                                          'Are you sure you want to force kill PID ${cp.pid} (${cp.name})?',
                                          () async {
                                            try {
                                              await provider.handleStopProject(
                                                  p.id);
                                              provider.showToast(
                                                  'Process ${cp.pid} terminated successfully.',
                                                  type: 'success');
                                            } catch (e) {
                                              provider.showToast(
                                                  'Failed to kill process: $e',
                                                  type: 'error');
                                            }
                                          },
                                        );
                                      },
                                    );
                                  }),
                              ],
                            ],
                          );
                        }).toList(),
                      ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildHeader(BuildContext context, int projectCount) {
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Row(
        children: [
          Icon(Icons.account_tree_outlined,
              size: 12, color: AppTheme.textSecondary(context)),
          const SizedBox(width: 4),
          Text('Process Tree',
              style: TextStyle(
                  color: AppTheme.textPrimary(context),
                  fontSize: 10,
                  fontWeight: FontWeight.w600)),
          const Spacer(),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
            decoration: BoxDecoration(
              color: AppTheme.textSecondary(context).withOpacity(0.1),
              borderRadius: BorderRadius.circular(3),
              border: Border.all(
                  color: AppTheme.borderPrimary(context), width: 0.5),
            ),
            child: Text('OS: Linux',
                style: TextStyle(
                    color: AppTheme.textSecondary(context), fontSize: 9)),
          ),
        ],
      ),
    );
  }

  Widget _buildEmptyState(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.copy_all_outlined,
              size: 40,
              color: AppTheme.textSecondary(context).withOpacity(0.4)),
          const SizedBox(height: 12),
          Text('No projects registered',
              style: TextStyle(
                  color: AppTheme.textSecondary(context), fontSize: 13)),
          const SizedBox(height: 4),
          Text('Add a project to get started',
              style: TextStyle(
                  color: AppTheme.textSecondary(context).withOpacity(0.6),
                  fontSize: 11)),
        ],
      ),
    );
  }

  Widget _buildEmptyProcessRow(BuildContext context) {
    return Container(
      padding: const EdgeInsets.only(left: 32, top: 4, bottom: 4),
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF12121A)
          : const Color(0xFFF0F0F4),
      child: Row(
        children: [
          Container(
            width: 1,
            height: 14,
            margin: const EdgeInsets.only(right: 8),
            color: AppTheme.borderPrimary(context),
          ),
          Text('Tracing processes in background...',
              style: TextStyle(
                  color: AppTheme.textSecondary(context).withOpacity(0.6),
                  fontSize: 10,
                  fontStyle: FontStyle.italic)),
        ],
      ),
    );
  }
}

/// Main project row in the process tree.
class _ProjectRow extends StatelessWidget {
  final Project project;
  final ProcessState? state;
  final bool isActive;
  final bool isRunning;
  final bool isExpanded;
  final double cpu;
  final double ram;
  final bool hasChildren;
  final VoidCallback onSelect;
  final VoidCallback onToggleExpand;
  final VoidCallback onStart;
  final VoidCallback onStop;
  final VoidCallback onForceKill;

  const _ProjectRow({
    required this.project,
    required this.state,
    required this.isActive,
    required this.isRunning,
    required this.isExpanded,
    required this.cpu,
    required this.ram,
    required this.hasChildren,
    required this.onSelect,
    required this.onToggleExpand,
    required this.onStart,
    required this.onStop,
    required this.onForceKill,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: isActive
            ? AppTheme.accentColor(context).withOpacity(0.08)
            : Colors.transparent,
        border: Border(
          bottom: BorderSide(
            color: AppTheme.borderPrimary(context).withOpacity(0.5),
            width: 0.5,
          ),
        ),
      ),
      child: Row(
        children: [
          // Expand button + Name
          Expanded(
            flex: 3,
            child: Row(
              children: [
                SizedBox(
                  width: 16,
                  child: hasChildren
                      ? GestureDetector(
                          onTap: onToggleExpand,
                          child: Icon(
                            isExpanded
                                ? Icons.expand_more
                                : Icons.chevron_right,
                            size: 14,
                            color: AppTheme.textSecondary(context),
                          ),
                        )
                      : const SizedBox(width: 14),
                ),
                GestureDetector(
                  onTap: onSelect,
                  child: Text(
                    project.name,
                    style: TextStyle(
                      color: isActive
                          ? AppTheme.accentColor(context)
                          : AppTheme.textPrimary(context),
                      fontSize: 11,
                      fontWeight:
                          isActive ? FontWeight.w600 : FontWeight.w500,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
          // Status
          Expanded(
            flex: 1,
            child: _StatusBadge(
              label: isRunning ? 'Running' : 'Stopped',
              isRunning: isRunning,
            ),
          ),
          // CPU
          Expanded(
            flex: 1,
            child: Text(
              isRunning ? '${cpu.toStringAsFixed(1)}%' : '0.0%',
              style: TextStyle(
                color: isRunning
                    ? (cpu > 80
                        ? AppColors.darkDanger
                        : cpu > 50
                            ? AppColors.darkWarning
                            : AppTheme.textSecondary(context))
                    : AppTheme.textSecondary(context).withOpacity(0.5),
                fontSize: 10,
                fontFamily: 'monospace',
              ),
            ),
          ),
          // RAM
          Expanded(
            flex: 1,
            child: Text(
              isRunning ? '${ram.toStringAsFixed(1)} MB' : '0.0 MB',
              style: TextStyle(
                color: isRunning
                    ? AppTheme.textSecondary(context)
                    : AppTheme.textSecondary(context).withOpacity(0.5),
                fontSize: 10,
                fontFamily: 'monospace',
              ),
            ),
          ),
          // Actions
          Expanded(
            flex: 2,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                if (isRunning)
                  _ActionChip(
                    label: 'Kill All',
                    color: AppColors.darkDanger,
                    onPressed: onStop,
                  )
                else
                  _ActionChip(
                    label: 'Run',
                    color: AppColors.darkSuccess,
                    onPressed: onStart,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Status badge pill.
class _StatusBadge extends StatelessWidget {
  final String label;
  final bool isRunning;

  const _StatusBadge({required this.label, required this.isRunning});

  @override
  Widget build(BuildContext context) {
    final color = isRunning ? AppColors.darkSuccess : AppColors.darkTextSecondary;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: color.withOpacity(0.3), width: 0.5),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 9,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

/// Small action button chip.
class _ActionChip extends StatelessWidget {
  final String label;
  final Color color;
  final VoidCallback onPressed;

  const _ActionChip({
    required this.label,
    required this.color,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onPressed,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
        decoration: BoxDecoration(
          color: color.withOpacity(0.15),
          borderRadius: BorderRadius.circular(3),
          border: Border.all(color: color.withOpacity(0.3), width: 0.5),
        ),
        child: Text(
          label,
          style: TextStyle(
            color: color,
            fontSize: 9,
            fontWeight: FontWeight.w600,
          ),
        ),
      ),
    );
  }
}

/// Child process row in the nested tree.
class _ChildProcessRow extends StatelessWidget {
  final ChildProcessInfo process;
  final bool isExpanded;
  final VoidCallback onToggleExpand;
  final VoidCallback onKill;

  const _ChildProcessRow({
    required this.process,
    required this.isExpanded,
    required this.onToggleExpand,
    required this.onKill,
  });

  Color _statusColor() {
    switch (process.status) {
      case 'Running':
        return const Color(0xFF10B981);
      case 'Sleeping':
        return const Color(0xFF3A86FF);
      case 'Stopped':
        return const Color(0xFFF59E0B);
      default:
        return AppColors.darkTextSecondary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final hasPorts = process.ports.isNotEmpty;
    return Column(
      children: [
        // Sub-process row
        Container(
          height: 28,
          padding: const EdgeInsets.only(left: 28, right: 8),
          decoration: BoxDecoration(
            color: Theme.of(context).brightness == Brightness.dark
                ? const Color(0xFF12121A)
                : const Color(0xFFF0F0F4),
            border: Border(
              bottom: BorderSide(
                color: AppTheme.borderPrimary(context).withOpacity(0.3),
                width: 0.5,
              ),
            ),
          ),
          child: Row(
            children: [
              // Tree connector + name
              Expanded(
                flex: 3,
                child: Row(
                  children: [
                    Container(
                      width: 1,
                      height: 14,
                      margin: const EdgeInsets.only(right: 6),
                      color: AppTheme.borderPrimary(context),
                    ),
                    GestureDetector(
                      onTap: onToggleExpand,
                      child: Icon(
                        isExpanded
                            ? Icons.expand_more
                            : Icons.chevron_right,
                        size: 12,
                        color: AppTheme.textSecondary(context),
                      ),
                    ),
                    const SizedBox(width: 2),
                    Text(
                      '[${process.pid}]',
                      style: TextStyle(
                        color: AppTheme.textSecondary(context),
                        fontSize: 10,
                        fontFamily: 'monospace',
                      ),
                    ),
                    const SizedBox(width: 4),
                    Flexible(
                      child: Tooltip(
                        message: process.cmd,
                        child: Text(
                          process.name,
                          style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 10,
                            fontWeight: FontWeight.w500,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              // Status with dot
              Expanded(
                flex: 1,
                child: Row(
                  children: [
                    Container(
                      width: 5,
                      height: 5,
                      margin: const EdgeInsets.only(right: 4),
                      decoration: BoxDecoration(
                        color: _statusColor(),
                        shape: BoxShape.circle,
                      ),
                    ),
                    Text(
                      process.status,
                      style: TextStyle(
                        color: _statusColor(),
                        fontSize: 9,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
              // CPU
              Expanded(
                flex: 1,
                child: Text(
                  '${process.cpuPercentage.toStringAsFixed(1)}%',
                  style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 10,
                    fontFamily: 'monospace',
                  ),
                ),
              ),
              // RAM
              Expanded(
                flex: 1,
                child: Text(
                  _formatBytes(process.ramBytes),
                  style: TextStyle(
                    color: AppTheme.textSecondary(context),
                    fontSize: 10,
                    fontFamily: 'monospace',
                  ),
                ),
              ),
              // Kill button
              Expanded(
                flex: 2,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    _ActionChip(
                      label: 'Kill',
                      color: AppColors.darkDanger,
                      onPressed: onKill,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        // Nested deep-dive details drawer
        if (isExpanded)
          Container(
            padding: const EdgeInsets.only(
                left: 40, right: 12, top: 8, bottom: 10),
            color: Theme.of(context).brightness == Brightness.dark
                ? const Color(0xFF0E0E16)
                : const Color(0xFFE8E8EE),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Command & CWD
                _DetailRow(
                  label: 'CWD:',
                  value: process.cwd,
                ),
                const SizedBox(height: 3),
                _DetailRow(
                  label: 'Command:',
                  value: process.cmd,
                  mono: true,
                  wordBreak: true,
                ),
                const SizedBox(height: 6),
                // Ports & Threads
                Row(
                  children: [
                    Row(
                      children: [
                        Icon(Icons.language,
                            size: 11,
                            color: hasPorts
                                ? AppColors.darkSuccess
                                : AppTheme.textSecondary(context)),
                        const SizedBox(width: 3),
                        Text('Ports: ',
                            style: TextStyle(
                                color: AppTheme.textSecondary(context),
                                fontSize: 10,
                                fontWeight: FontWeight.w600)),
                        if (hasPorts)
                          ...process.ports.map(
                            (port) => Container(
                              margin:
                                  const EdgeInsets.only(right: 3),
                              padding:
                                  const EdgeInsets.symmetric(
                                      horizontal: 4, vertical: 1),
                              decoration: BoxDecoration(
                                color: AppColors.darkSuccess
                                    .withOpacity(0.15),
                                borderRadius:
                                    BorderRadius.circular(3),
                              ),
                              child: Text(
                                ':$port',
                                style: TextStyle(
                                  color: AppColors.darkSuccess,
                                  fontSize: 10,
                                  fontFamily: 'monospace',
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                            ),
                          )
                        else
                          Text('No active ports',
                              style: TextStyle(
                                  color: AppTheme.textSecondary(context)
                                      .withOpacity(0.6),
                                  fontSize: 10)),
                      ],
                    ),
                    const SizedBox(width: 16),
                    Row(
                      children: [
                        Icon(Icons.layers_outlined,
                            size: 11,
                            color: AppTheme.accentColor(context)),
                        const SizedBox(width: 3),
                        Text('Threads: ',
                            style: TextStyle(
                                color: AppTheme.textSecondary(context),
                                fontSize: 10,
                                fontWeight: FontWeight.w600)),
                        Text('${process.threadCount} active',
                            style: TextStyle(
                                color: AppTheme.textPrimary(context),
                                fontSize: 10,
                                fontFamily: 'monospace')),
                      ],
                    ),
                  ],
                ),
                const SizedBox(height: 6),
                // Loaded modules / Assets
                Row(
                  children: [
                    Icon(Icons.storage_outlined,
                        size: 11,
                        color: AppTheme.textSecondary(context)),
                    const SizedBox(width: 3),
                    Text('Loaded Libraries & Assets',
                        style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontSize: 10,
                            fontWeight: FontWeight.w600)),
                  ],
                ),
                const SizedBox(height: 4),
                if (process.loadedModules.isNotEmpty)
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.all(4),
                    constraints: const BoxConstraints(maxHeight: 60),
                    decoration: BoxDecoration(
                      color: Theme.of(context).brightness == Brightness.dark
                          ? AppColors.darkSurface
                          : AppColors.lightSurface,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                          color: AppTheme.borderPrimary(context)),
                    ),
                    child: SingleChildScrollView(
                      child: Wrap(
                        spacing: 4,
                        runSpacing: 2,
                        children: process.loadedModules.map((mod) {
                          final filename =
                              mod.split('/').last;
                          return Tooltip(
                            message: mod,
                            child: Container(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 4, vertical: 1),
                              decoration: BoxDecoration(
                                color: AppTheme.textSecondary(context)
                                    .withOpacity(0.08),
                                borderRadius:
                                    BorderRadius.circular(2),
                              ),
                              child: Text(
                                filename,
                                style: TextStyle(
                                  color: AppTheme.textSecondary(context),
                                  fontSize: 9,
                                  fontFamily: 'monospace',
                                ),
                              ),
                            ),
                          );
                        }).toList(),
                      ),
                    ),
                  )
                else
                  Text('No system maps loaded',
                      style: TextStyle(
                          color: AppTheme.textSecondary(context)
                              .withOpacity(0.6),
                          fontSize: 10,
                          fontStyle: FontStyle.italic)),
              ],
            ),
          ),
      ],
    );
  }

  String _formatBytes(int bytes) {
    if (bytes >= 1073741824) {
      return '${(bytes / 1073741824).toStringAsFixed(2)} GB';
    } else if (bytes >= 1048576) {
      return '${(bytes / 1048576).toStringAsFixed(1)} MB';
    } else if (bytes >= 1024) {
      return '${(bytes / 1024).toStringAsFixed(1)} KB';
    }
    return '$bytes B';
  }
}

/// Detail row for the expanded process drawer.
class _DetailRow extends StatelessWidget {
  final String label;
  final String value;
  final bool mono;
  final bool wordBreak;

  const _DetailRow({
    required this.label,
    required this.value,
    this.mono = false,
    this.wordBreak = false,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('$label ',
            style: TextStyle(
                color: AppTheme.textSecondary(context).withOpacity(0.7),
                fontSize: 10,
                fontWeight: FontWeight.w600)),
        Expanded(
          child: Text(
            value,
            style: TextStyle(
              color: AppTheme.textPrimary(context),
              fontSize: 10,
              fontFamily: mono ? 'monospace' : null,
            ),
          ),
        ),
      ],
    );
  }
}
