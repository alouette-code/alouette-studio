import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:window_manager/window_manager.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';
import '../models/project.dart';

class Header extends StatelessWidget {
  final String theme;
  final VoidCallback onToggleTheme;
  final String searchQuery;
  final void Function(String) onSearchChanged;
  final Project? activeProject;
  final ProcessState activeState;
  final void Function(String action, dynamic payload) onFileAction;
  final List<Map<String, dynamic>> agentHistoryList;
  final void Function(String sessionId) onLoadAgentSession;

  const Header({
    super.key,
    required this.theme,
    required this.onToggleTheme,
    required this.searchQuery,
    required this.onSearchChanged,
    this.activeProject,
    required this.activeState,
    required this.onFileAction,
    required this.agentHistoryList,
    required this.onLoadAgentSession,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = theme == 'dark';

    return DragToMoveArea(
      child: Container(
      height: 40,
      decoration: BoxDecoration(
        color: isDark ? const Color(0xFF111115) : const Color(0xFFFFFFFF),
        border: Border(
          bottom: BorderSide(color: isDark ? const Color(0xFF22222A) : const Color(0xFFC9C9D4)),
        ),
      ),
      child: Row(
        children: [
          const SizedBox(width: 12),
          // Left: Brand Logo & Menus
          MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              onTap: () => onFileAction('open-welcome', null),
              child: Image.asset(
                'assets/logo_alouette.png',
                width: 22,
                height: 22,
                fit: BoxFit.contain,
              ),
            ),
          ),
          const SizedBox(width: 12),
          _HeaderDropdownBtn(
            icon: Icons.description_outlined,
            label: 'File',
            onTap: () => _showFileMenu(context),
            isDark: isDark,
          ),
          _HeaderDropdownBtn(
            icon: Icons.settings_outlined,
            label: 'Setting',
            onTap: () => _showSettingMenu(context),
            isDark: isDark,
          ),

          const Spacer(),

          // Center: Search Bar
          Material(
            color: Colors.transparent,
            child: Container(
              width: 360,
              height: 26,
              decoration: BoxDecoration(
                color: isDark ? const Color(0xFF0A0A0C) : const Color(0xFFF0F0F3),
                borderRadius: BorderRadius.zero,
                border: Border.all(color: isDark ? const Color(0xFF22222A) : const Color(0xFFC9C9D4)),
              ),
              child: TextField(
                onChanged: onSearchChanged,
                controller: TextEditingController.fromValue(TextEditingValue(text: searchQuery)),
                style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 11.5, height: 1.4),
                decoration: InputDecoration(
                  hintText: 'Search processes, logs, commands...',
                  hintStyle: TextStyle(color: AppTheme.textSecondary(context), fontSize: 11.5),
                  prefixIcon: Icon(Icons.search, size: 13, color: AppTheme.textSecondary(context)),
                  suffixIcon: searchQuery.isNotEmpty
                      ? GestureDetector(
                          onTap: () => onSearchChanged(''),
                          child: Icon(Icons.close, size: 13, color: AppTheme.textSecondary(context)),
                        )
                      : null,
                  border: InputBorder.none,
                  contentPadding: const EdgeInsets.symmetric(vertical: 0, horizontal: 8),
                  isDense: true,
                ),
              ),
            ),
          ),

          const Spacer(),

          // Right: Active project controls, Database resources, Brand logo, Theme toggle, Window actions
          if (activeProject != null) ...[
            Container(
              height: double.infinity,
              padding: const EdgeInsets.only(right: 12),
              decoration: BoxDecoration(
                border: Border(
                  right: BorderSide(color: isDark ? const Color(0xFF22222A) : const Color(0xFFC9C9D4)),
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // Cloudflare Tunnel
                  GestureDetector(
                    onTap: () => onFileAction('toggle-tunnel', activeProject!.id),
                    child: Tooltip(
                      message: activeProject!.enableTunnel
                          ? 'Cloudflare Tunnel Enabled'
                          : 'Cloudflare Tunnel Disabled',
                      child: Icon(
                        activeProject!.enableTunnel ? Icons.cloud_done : Icons.cloud_queue_rounded,
                        size: 15,
                        color: activeProject!.enableTunnel ? const Color(0xFFF38020) : AppTheme.textSecondary(context).withValues(alpha: 0.55),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    activeProject!.name,
                    style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.bold,
                      color: AppTheme.textPrimary(context),
                    ),
                  ),
                  const SizedBox(width: 8),
                  _HeaderMetaCapsule(
                    label: 'PID',
                    value: activeState.type == ProcessStateType.running
                        ? (activeState.data is Map ? (activeState.data['pid']?.toString() ?? 'N/A') : activeState.data.toString())
                        : 'N/A',
                    isDark: isDark,
                  ),
                  if (activeProject!.port != null) ...[
                    const SizedBox(width: 8),
                    _HeaderMetaCapsule(
                      label: 'PORT',
                      value: activeProject!.port.toString(),
                      valueColor: const Color(0xFF22C55E),
                      isDark: isDark,
                    ),
                  ],
                  const SizedBox(width: 8),
                  _HeaderProcessBtn(
                    isStop: activeState.type == ProcessStateType.running || activeState.type == ProcessStateType.setup,
                    onTap: () {
                      if (activeState.type == ProcessStateType.running || activeState.type == ProcessStateType.setup) {
                        onFileAction('stop', activeProject!.id);
                      } else {
                        onFileAction('start', activeProject!.id);
                      }
                    },
                    isDark: isDark,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
          ],

          // Database resources icon
          MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              onTap: () => onFileAction('open-resources', null),
              child: Tooltip(
                message: 'Tài nguyên',
                child: Icon(
                  Icons.storage_rounded,
                  size: 14,
                  color: AppTheme.textSecondary(context),
                ),
              ),
            ),
          ),
          const SizedBox(width: 12),

          // Small Brand logo
          MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              onTap: () => onFileAction('open-welcome', null),
              child: Image.asset(
                'assets/logo_alouette.png',
                width: 14,
                height: 14,
                fit: BoxFit.contain,
              ),
            ),
          ),
          const SizedBox(width: 12),

          // Theme toggle
          MouseRegion(
            cursor: SystemMouseCursors.click,
            child: GestureDetector(
              onTap: onToggleTheme,
              child: Icon(
                isDark ? Icons.wb_sunny_outlined : Icons.mode_night_outlined,
                size: 14,
                color: AppTheme.textSecondary(context),
              ),
            ),
          ),
          const SizedBox(width: 12),

          // Window action controls - dùng _WindowControls để cache maximized state
          const _WindowControls(),
        ],
      ),
    ),
    );
  }

  void _showFileMenu(BuildContext context) {
    showMenu(
      context: context,
      position: const RelativeRect.fromLTRB(60, 40, 60, 40),
      color: Theme.of(context).brightness == Brightness.dark ? const Color(0xFF1E1E2A) : Colors.white,
      items: <PopupMenuEntry<String>>[
        PopupMenuItem(value: 'new-text-file', child: Text('New Text File', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'new-file', child: Text('New File...', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'open-file', child: Text('Open File...', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'open-folder', child: Text('Open Folder...', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        const PopupMenuDivider(),
        PopupMenuItem(value: 'save', child: Text('Save', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'save-as', child: Text('Save As...', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'save-all', child: Text('Save All', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'revert', child: Text('Revert File', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        const PopupMenuDivider(),
        PopupMenuItem(value: 'close-editor', child: Text('Close Editor', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        PopupMenuItem(value: 'close-window', child: Text('Close Window', style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)))),
        const PopupMenuItem(value: 'exit', child: Text('Exit', style: TextStyle(fontSize: 12, color: AppColors.darkDanger))),
      ],
    ).then((value) {
      if (value != null) onFileAction(value, null);
    });
  }

  void _showSettingMenu(BuildContext context) {
    final provider = context.read<AppProvider>();
    showMenu(
      context: context,
      position: const RelativeRect.fromLTRB(140, 40, 140, 40),
      color: Theme.of(context).brightness == Brightness.dark ? const Color(0xFF1E1E2A) : Colors.white,
      items: <PopupMenuEntry<String>>[
        const PopupMenuItem(
          enabled: false,
          child: Text(
            'Buffer Settings',
            style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: Colors.grey),
          ),
        ),
        PopupMenuItem(
          value: 'capped-2000',
          child: Text(
            'Capped 2000 lines',
            style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)),
          ),
        ),
        const PopupMenuDivider(),
        const PopupMenuItem(
          enabled: false,
          child: Text(
            'System Style',
            style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: Colors.grey),
          ),
        ),
        PopupMenuItem(
          value: 'toggle-theme',
          child: Text(
            'Toggle Theme',
            style: TextStyle(fontSize: 12, color: AppTheme.textPrimary(context)),
          ),
        ),
      ],
    ).then((value) {
      if (value == 'toggle-theme') {
        onToggleTheme();
      } else if (value == 'capped-2000') {
        provider.showToast('Log buffer capped at 2000 lines.', type: 'info');
      }
    });
  }
}

// ── Widget Window Controls (Minimize/Maximize/Close) với cached state ──
// Dùng WindowListener để cache isMaximized → không cần async query khi nhấn nút
class _WindowControls extends StatefulWidget {
  const _WindowControls();

  @override
  State<_WindowControls> createState() => _WindowControlsState();
}

class _WindowControlsState extends State<_WindowControls> with WindowListener {
  bool _isMaximized = false;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    // Lấy trạng thái ban đầu một lần duy nhất
    windowManager.isMaximized().then((val) {
      if (mounted) setState(() => _isMaximized = val);
    });
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  // WindowListener callbacks - cập nhật cache ngay lập tức
  @override
  void onWindowMaximize() => setState(() => _isMaximized = true);

  @override
  void onWindowUnmaximize() => setState(() => _isMaximized = false);

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return SizedBox(
      height: double.infinity,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          _HeaderWindowButton(
            icon: Icons.remove,
            // Fire-and-forget: không await, không block UI
            onTap: () => windowManager.minimize(),
            isDark: isDark,
          ),
          _HeaderWindowButton(
            icon: _isMaximized
                ? Icons.filter_none_outlined   // restore icon
                : Icons.crop_square_outlined,  // maximize icon
            // Dùng cached _isMaximized thay vì async query
            onTap: () {
              if (_isMaximized) {
                windowManager.unmaximize();
              } else {
                windowManager.maximize();
              }
            },
            isDark: isDark,
          ),
          _HeaderWindowButton(
            icon: Icons.close,
            onTap: () => windowManager.close(),
            isClose: true,
            isDark: isDark,
          ),
        ],
      ),
    );
  }
}

// ── Widget Dropdown button File / Setting ──
class _HeaderDropdownBtn extends StatefulWidget {

  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final bool isDark;

  const _HeaderDropdownBtn({
    required this.icon,
    required this.label,
    required this.onTap,
    required this.isDark,
  });

  @override
  State<_HeaderDropdownBtn> createState() => _HeaderDropdownBtnState();
}

class _HeaderDropdownBtnState extends State<_HeaderDropdownBtn> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final normalColor = widget.isDark ? const Color(0xFF92929E) : const Color(0xFF535360);
    final hoverColor = widget.isDark ? const Color(0xFFE6E6EB) : const Color(0xFF121216);
    final hoverBg = widget.isDark ? const Color(0xFF1B1B22) : const Color(0xFFE3E3E8);

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          height: double.infinity,
          color: _isHovered ? hoverBg : Colors.transparent,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(widget.icon, size: 14, color: _isHovered ? hoverColor : normalColor),
              const SizedBox(width: 6),
              Text(
                widget.label,
                style: TextStyle(
                  color: _isHovered ? hoverColor : normalColor,
                  fontSize: 11.5,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── Widget Window Control Button (Minimize, Maximize, Close) ──
class _HeaderWindowButton extends StatefulWidget {
  final IconData icon;
  final VoidCallback onTap;
  final bool isClose;
  final bool isDark;

  const _HeaderWindowButton({
    required this.icon,
    required this.onTap,
    this.isClose = false,
    required this.isDark,
  });

  @override
  State<_HeaderWindowButton> createState() => _HeaderWindowButtonState();
}

class _HeaderWindowButtonState extends State<_HeaderWindowButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final normalColor = widget.isDark ? const Color(0xFF8888A0) : const Color(0xFF6B6B80);
    final hoverColor = widget.isClose ? Colors.white : (widget.isDark ? const Color(0xFFE8E8EF) : const Color(0xFF1A1A24));

    final hoverBg = widget.isClose
        ? const Color(0xFFEF4444)
        : (widget.isDark ? const Color(0xFF1B1B22) : const Color(0xFFE3E3E8));

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          width: 46,
          height: double.infinity,
          color: _isHovered ? hoverBg : Colors.transparent,
          child: Center(
            child: Icon(
              widget.icon,
              size: widget.isClose ? 14 : 12,
              color: _isHovered ? hoverColor : normalColor,
            ),
          ),
        ),
      ),
    );
  }
}

// ── Widget PID / PORT Capsule ──
class _HeaderMetaCapsule extends StatelessWidget {
  final String label;
  final String value;
  final Color? valueColor;
  final bool isDark;

  const _HeaderMetaCapsule({
    required this.label,
    required this.value,
    this.valueColor,
    required this.isDark,
  });

  @override
  Widget build(BuildContext context) {
    final borderCol = isDark ? const Color(0xFF22222A) : const Color(0xFFC9C9D4);
    final bgCol = isDark ? const Color(0xFF0A0A0C) : const Color(0xFFF0F0F3);
    final labelCol = isDark ? const Color(0xFF585866) : const Color(0xFF828292); // var(--text-muted)
    final valCol = valueColor ?? (isDark ? const Color(0xFF92929E) : const Color(0xFF535360)); // var(--text-secondary)

    return Container(
      height: 20,
      padding: const EdgeInsets.symmetric(horizontal: 6),
      decoration: BoxDecoration(
        color: bgCol,
        border: Border.all(color: borderCol),
        borderRadius: BorderRadius.zero,
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Text(
            label,
            style: TextStyle(
              color: labelCol,
              fontSize: 10,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(width: 4),
          Text(
            value,
            style: TextStyle(
              color: valCol,
              fontSize: 10,
              fontWeight: FontWeight.w700,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }
}

// ── Widget Start / Stop Process Button ──
class _HeaderProcessBtn extends StatefulWidget {
  final bool isStop;
  final VoidCallback onTap;
  final bool isDark;

  const _HeaderProcessBtn({
    required this.isStop,
    required this.onTap,
    required this.isDark,
  });

  @override
  State<_HeaderProcessBtn> createState() => _HeaderProcessBtnState();
}

class _HeaderProcessBtnState extends State<_HeaderProcessBtn> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final Color normalBg;
    final Color hoverBg;
    if (widget.isStop) {
      normalBg = const Color(0xFFEF4444);
      hoverBg = const Color(0xFFDC2626);
    } else {
      normalBg = widget.isDark ? const Color(0xFF6366F1) : const Color(0xFF0056E0);
      hoverBg = widget.isDark ? const Color(0xFF5496FF) : const Color(0xFF0047B8);
    }

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          height: 22,
          padding: const EdgeInsets.symmetric(horizontal: 8),
          decoration: BoxDecoration(
            color: _isHovered ? hoverBg : normalBg,
            borderRadius: BorderRadius.zero,
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                widget.isStop ? Icons.square_rounded : Icons.play_arrow_rounded,
                size: 10,
                color: Colors.white,
              ),
              const SizedBox(width: 4),
              Text(
                widget.isStop ? 'Stop' : 'Start',
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 10,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
