import 'package:flutter/material.dart';
import '../theme/app_theme.dart';
import '../models/project.dart';

/// Left sidebar project tab list mirroring TabList.tsx.
///
/// Displays a filtered list of projects with status dots, names,
/// active state highlighting, and delete buttons.
class TabListWidget extends StatelessWidget {
  final List<Project> projects;
  final String activeProjectId;
  final Map<String, ProcessState> projectStates;
  final void Function(String id) onSelectProject;
  final void Function(String id) onDeleteProject;

  const TabListWidget({
    super.key,
    required this.projects,
    required this.activeProjectId,
    required this.projectStates,
    required this.onSelectProject,
    required this.onDeleteProject,
  });

  Color _statusColor(ProcessStateType type) {
    switch (type) {
      case ProcessStateType.stopped:
        return AppColors.darkTextSecondary;
      case ProcessStateType.setup:
        return AppColors.darkInfo;
      case ProcessStateType.running:
        return AppColors.darkSuccess;
      case ProcessStateType.crashing:
        return AppColors.darkWarning;
      case ProcessStateType.terminated:
        return AppColors.darkTextSecondary;
      case ProcessStateType.fatal:
        return AppColors.darkDanger;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      color: isDark ? const Color(0xFF111118) : const Color(0xFFF5F5F8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            child: Row(
              children: [
                Icon(
                  Icons.grid_view_rounded,
                  size: 12,
                  color: AppTheme.textSecondary(context),
                ),
                const SizedBox(width: 6),
                Text(
                  'Projects',
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                    color: AppTheme.textSecondary(context),
                    letterSpacing: 0.5,
                  ),
                ),
              ],
            ),
          ),
          const Divider(height: 1),

          // Tab list
          Expanded(
            child: projects.isEmpty
                ? Center(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            Icons.copy_all_outlined,
                            size: 24,
                            color: AppTheme.textSecondary(context).withOpacity(0.3),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'No active tabs.',
                            style: TextStyle(
                              fontSize: 11,
                              color: AppTheme.textSecondary(context).withOpacity(0.6),
                            ),
                          ),
                        ],
                      ),
                    ),
                  )
                : ListView.separated(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    itemCount: projects.length,
                    separatorBuilder: (_, __) => const SizedBox(height: 1),
                    itemBuilder: (context, index) {
                      final project = projects[index];
                      final state = projectStates[project.id] ?? ProcessState(type: ProcessStateType.stopped);
                      final isActive = project.id == activeProjectId;

                      return _TabListItem(
                        project: project,
                        state: state,
                        isActive: isActive,
                        statusColor: _statusColor(state.type),
                        isDark: isDark,
                        onTap: () {
                          onSelectProject(project.id);
                        },
                        onDelete: () {
                          onDeleteProject(project.id);
                        },
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _TabListItem extends StatelessWidget {
  final Project project;
  final ProcessState state;
  final bool isActive;
  final Color statusColor;
  final bool isDark;
  final VoidCallback onTap;
  final VoidCallback onDelete;

  const _TabListItem({
    required this.project,
    required this.state,
    required this.isActive,
    required this.statusColor,
    required this.isDark,
    required this.onTap,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: isActive
              ? (isDark ? const Color(0xFF1E1E2E) : const Color(0xFFE8E8F0))
              : null,
          border: isActive
              ? const Border(
                  left: BorderSide(color: AppColors.darkAccent, width: 2),
                )
              : null,
        ),
        child: Row(
          children: [
            // Status dot
            Container(
              width: 7,
              height: 7,
              decoration: BoxDecoration(
                color: statusColor,
                shape: BoxShape.circle,
                boxShadow: [
                  BoxShadow(
                    color: statusColor.withOpacity(0.3),
                    blurRadius: 3,
                    spreadRadius: 0.5,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),

            // Project name
            Expanded(
              child: Text(
                project.name,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: isActive ? FontWeight.w600 : FontWeight.w400,
                  color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),

            // State label
            if (state.type != ProcessStateType.stopped)
              Padding(
                padding: const EdgeInsets.only(right: 4),
                child: Text(
                  state.type.label,
                  style: TextStyle(
                    fontSize: 8,
                    fontWeight: FontWeight.w600,
                    color: statusColor,
                  ),
                ),
              ),

            // Delete button
            GestureDetector(
              onTap: onDelete,
              child: Container(
                width: 20,
                height: 20,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: isDark ? AppColors.darkHover : AppColors.lightHover,
                  borderRadius: BorderRadius.circular(3),
                ),
                child: Icon(
                  Icons.delete_outline_rounded,
                  size: 12,
                  color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
