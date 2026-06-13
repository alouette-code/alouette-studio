import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'providers/app_provider.dart';
import 'theme/app_theme.dart';
import 'components/header.dart';
import 'components/welcome_page.dart';
import 'components/tab_list.dart';
import 'components/file_explorer.dart';
import 'components/code_editor.dart';
import 'components/terminal_panel.dart';
import 'components/config_setup.dart';
import 'components/process_manager.dart';
import 'components/project_resources.dart';
import 'components/cloudflare_tunnel.dart';
import 'components/environment_setup.dart';
import 'components/ai_agent.dart';
import 'components/git_panel.dart';
import 'components/mini_postman.dart';
import 'components/sqlite_editor.dart';
import 'components/admin_panel.dart';
import 'components/window_resizer.dart';
import 'widgets/resizable_handle.dart';
import 'widgets/toast_overlay.dart';
import 'widgets/confirm_dialog.dart';
import 'widgets/file_prompt_dialog.dart';
import 'widgets/context_menu.dart';

class AlouetteStudioHome extends StatefulWidget {
  const AlouetteStudioHome({super.key});

  @override
  State<AlouetteStudioHome> createState() => _AlouetteStudioHomeState();
}

class _AlouetteStudioHomeState extends State<AlouetteStudioHome> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<AppProvider>().loadProjects();
    });
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        return Stack(
          children: [
            Column(
              children: [
                // 1. Header
                const _HeaderBar(),

                // 2. Main workspace
                Expanded(
                  child: provider.activeProjectId.isEmpty
                      ? const WelcomePage()
                      : const _WorkspaceGrid(),
                ),

                // 3. Footer navbar
                const _FooterNavbar(),
              ],
            ),

            // Overlays
            const ToastOverlay(),
            const ConfirmDialogOverlay(),
            const FilePromptOverlay(),
          ],
        );
      },
    );
  }
}

/// Header bar (mirrors Header.tsx)
class _HeaderBar extends StatelessWidget {
  const _HeaderBar();

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) => Header(
        theme: provider.isDark ? 'dark' : 'light',
        onToggleTheme: provider.toggleTheme,
        searchQuery: provider.searchQuery,
        onSearchChanged: provider.setSearchQuery,
        activeProject: provider.activeProject,
        activeState: provider.activeState,
        onFileAction: (action, payload) => _handleFileAction(context, action, payload),
        agentHistoryList: const [],
        onLoadAgentSession: (_) {},
      ),
    );
  }

  Future<void> _handleFileAction(BuildContext context, String action, dynamic payload) async {
    final provider = context.read<AppProvider>();
    switch (action) {
      case 'open-welcome':
        provider.activeProjectId = '';
        break;
      case 'new-text-file':
        provider.showFilePrompt('New Text File', 'Enter file path/name...', 'untitled.txt', (name) {
          // TODO: create file via FRB
          provider.showToast('Created $name', type: 'success');
        });
        break;
      case 'open-file':
        // TODO: open file dialog via FRB
        break;
      case 'open-folder':
        // TODO: open folder dialog via FRB
        break;
      case 'save':
        provider.showToast('Saved', type: 'success');
        break;
      case 'save-as':
        provider.showToast('Save As...', type: 'info');
        break;
      case 'close-editor':
        if (provider.activePaneFilePath != null) {
          provider.handleFileClose(provider.activePaneIndex, provider.activePaneFilePath!);
        }
        break;
      case 'toggle-tunnel':
        provider.handleFileOpen('__cloudflare_tunnel__');
        break;
      case 'open-resources':
        provider.handleFileOpen('__resources__');
        break;
      case 'exit':
        exit(0);
    }
  }
}

/// Main workspace 3-column grid (mirrors workspace-grid in App.tsx)
class _WorkspaceGrid extends StatelessWidget {
  const _WorkspaceGrid();

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        return LayoutBuilder(
          builder: (context, constraints) {
            // Calculate column widths
            final leftW = provider.isLeftSidebarOpen ? provider.leftSidebarWidth : 0.0;
            final rightW = provider.isRightSidebarOpen ? provider.rightSidebarWidth : 0.0;
            final centerW = constraints.maxWidth - leftW - rightW;

            return Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                // ── LEFT COLUMN ──
                if (provider.isLeftSidebarOpen)
                  SizedBox(
                    width: provider.leftSidebarWidth,
                    child: const _LeftColumn(),
                  ),

                // ── CENTER COLUMN ──
                SizedBox(
                  width: centerW > 0 ? centerW : constraints.maxWidth - leftW - rightW,
                  child: const _CenterColumn(),
                ),

                // ── RIGHT COLUMN ──
                if (provider.isRightSidebarOpen)
                  SizedBox(
                    width: provider.rightSidebarWidth,
                    child: const _RightColumn(),
                  ),
              ],
            );
          },
        );
      },
    );
  }
}

/// Left column: TabList (top) + FileExplorer (bottom)
class _LeftColumn extends StatelessWidget {
  const _LeftColumn();

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();
    return Container(
      decoration: BoxDecoration(
        border: Border(
          right: BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Column(
        children: [
          // Zone 1: Tab List
          SizedBox(
            height: provider.tabListHeight,
            child: Column(
              children: [
                Expanded(
                  child: TabListWidget(
                    projects: provider.filteredProjects,
                    activeProjectId: provider.activeProjectId,
                    projectStates: provider.projectStates,
                    onSelectProject: (id) => provider.activeProjectId = id,
                    onDeleteProject: provider.handleDeleteProject,
                  ),
                ),
                // Resize handle
                HorizontalResizeHandle(
                  onResize: (delta) {
                    provider.tabListHeight += delta;
                  },
                ),
              ],
            ),
          ),
          // Zone: File Explorer
          const Expanded(
            child: FileExplorerWidget(),
          ),
        ],
      ),
    );
  }
}

/// Center column: Code Editor (top) + Terminal Panel (bottom)
class _CenterColumn extends StatelessWidget {
  const _CenterColumn();

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();
    return Column(
      children: [
        // Zone 2: Code Editor
        Expanded(
          flex: provider.isBottomPanelOpen ? 2 : 3,
          child: Container(
            decoration: provider.isBottomPanelOpen
                ? BoxDecoration(
                    border: Border(
                      bottom: BorderSide(color: AppTheme.borderPrimary(context)),
                    ),
                  )
                : null,
            child: const _EditorPaneContent(),
          ),
        ),
        // Zone 4: Terminal
        if (provider.isBottomPanelOpen)
          const Expanded(
            flex: 1,
            child: TerminalPanelWidget(),
          ),
      ],
    );
  }
}

/// Editor pane content with split support
class _EditorPaneContent extends StatelessWidget {
  const _EditorPaneContent();

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();
    final panes = provider.panes;
    final activeIdx = provider.activePaneIndex;

    if (panes.length == 1) {
      return _buildEditorPane(context, panes[0], 0, true);
    }

    return Row(
      children: panes.asMap().entries.map((entry) {
        final idx = entry.key;
        final pane = entry.value;
        final isActive = idx == activeIdx;
        return Expanded(
          child: Container(
            decoration: BoxDecoration(
              border: idx > 0
                  ? Border(
                      left: BorderSide(color: AppTheme.borderPrimary(context)),
                    )
                  : null,
            ),
            child: _buildEditorPane(context, pane, idx, isActive),
          ),
        );
      }).toList(),
    );
  }

  Widget _buildEditorPane(BuildContext context, EditorPane pane, int idx, bool isActive) {
    final provider = context.read<AppProvider>();
    final filePath = pane.openFilePath;

    if (filePath == null) {
      return _buildEmptyEditor(context, idx);
    }

    // Check for special virtual files
    if (filePath == '__resources__') {
      return ProjectResourcesWidget(
        activeProject: provider.activeProject,
        activeState: provider.activeState,
        resourceHistory: provider.resourceHistory,
      );
    }
    if (filePath == '__cloudflare_tunnel__') {
      return const CloudflareTunnelWidget();
    }
    if (filePath == '__environment__') {
      return EnvironmentSetupWidget(activeProject: provider.activeProject);
    }
    if (filePath.startsWith('__agent_history__:')) {
      return AiAgentTabWidget(filePath: filePath);
    }
    if (filePath.endsWith('.db') || filePath.endsWith('.sqlite') || filePath.endsWith('.sqlite3')) {
      return SqliteEditorWidget(filePath: filePath);
    }

    return CodeEditorWidget(
      filePath: filePath,
      content: provider.filesContent[filePath] ?? '',
    );
  }

  Widget _buildEmptyEditor(BuildContext context, int paneIdx) {
    final provider = context.read<AppProvider>();
    return GestureDetector(
      onSecondaryTap: () => provider.showConfirm('Split screen?', provider.handleSplit),
      child: Container(
        color: Theme.of(context).brightness == Brightness.dark
            ? const Color(0xFF14141E)
            : const Color(0xFFFAFAFC),
        child: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(Icons.insert_drive_file_outlined,
                  size: 32,
                  color: AppTheme.textSecondary(context)),
              const SizedBox(height: 12),
              Text('No File Selected',
                  style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              Text(
                'Click on any file in the Project Explorer',
                style: TextStyle(
                  fontSize: 11,
                  color: AppTheme.textSecondary(context),
                ),
              ),
              if (provider.panes.length > 1) ...[
                const SizedBox(height: 16),
                TextButton(
                  onPressed: () => provider.handleClosePane(paneIdx),
                  child: const Text('Close this pane', style: TextStyle(fontSize: 11)),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Right column: ConfigSetup (top) + ProcessManager (bottom) or AiAgent or GitPanel
class _RightColumn extends StatelessWidget {
  const _RightColumn();

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();

    return Container(
      decoration: BoxDecoration(
        border: Border(
          left: BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Column(
        children: [
          if (provider.isAiViewActive)
            const Expanded(child: AiAgentWidget())
          else if (provider.isGitViewActive)
            const Expanded(child: GitPanelWidget())
          else ...[
            // Zone 3: Config Setup
            SizedBox(
              height: provider.configHeight,
              child: Column(
                children: [
                  const Expanded(
                    child: ConfigSetupWidget(),
                  ),
                  HorizontalResizeHandle(
                    onResize: (delta) => provider.configHeight += delta,
                  ),
                ],
              ),
            ),
            // Zone 5: Process Manager
            const Expanded(
              child: ProcessManagerWidget(),
            ),
          ],
        ],
      ),
    );
  }
}

/// Footer navbar with tool buttons
class _FooterNavbar extends StatelessWidget {
  const _FooterNavbar();

  @override
  Widget build(BuildContext context) {
    final provider = context.watch<AppProvider>();
    return Container(
      height: 32,
      decoration: BoxDecoration(
        color: Theme.of(context).brightness == Brightness.dark
            ? const Color(0xFF0D0D14)
            : const Color(0xFFE8E8EE),
        border: Border(
          top: BorderSide(color: AppTheme.borderPrimary(context)),
        ),
      ),
      child: Row(
        children: [
          // Nav tabs
          Expanded(
            child: Row(
              children: [
                _NavTab(
                  icon: Icons.grid_view_rounded,
                  isActive: provider.activeProjectId.isNotEmpty,
                  onTap: () {
                    if (provider.filteredProjects.isNotEmpty) {
                      provider.activeProjectId = provider.filteredProjects.first.id;
                    }
                  },
                ),
                _NavTab(
                  icon: Icons.terminal_rounded,
                  onTap: () {
                    // Ensure bottom panel is open and scroll to terminal
                    if (!provider.isBottomPanelOpen) {
                      provider.toggleBottomPanel();
                    }
                    provider.showToast('Terminal', type: 'info');
                  },
                ),
                Container(
                  width: 1,
                  height: 16,
                  color: AppTheme.borderPrimary(context),
                  margin: const EdgeInsets.symmetric(horizontal: 8),
                ),
                _NavTab(
                  icon: Icons.view_sidebar_outlined,
                  isActive: provider.isLeftSidebarOpen,
                  size: 15,
                  onTap: provider.toggleLeftSidebar,
                ),
                _NavTab(
                  icon: Icons.view_agenda_outlined,
                  isActive: provider.isBottomPanelOpen,
                  size: 15,
                  onTap: provider.toggleBottomPanel,
                ),
                _NavTab(
                  icon: Icons.view_sidebar_outlined,
                  isActive: provider.isRightSidebarOpen,
                  size: 15,
                  onTap: provider.toggleRightSidebar,
                  flip: true,
                ),
              ],
            ),
          ),

          // Tool buttons
          Row(
            children: [
              _ToolButton(
                icon: Icons.account_tree_outlined,
                isActive: provider.isGitViewActive && provider.isRightSidebarOpen,
                onTap: () {
                  if (!provider.isRightSidebarOpen) {
                    provider.toggleRightSidebar();
                    if (!provider.isGitViewActive) {
                      provider.setGitViewActive(true);
                    }
                  } else {
                    if (provider.isGitViewActive) {
                      final wasAi = provider.wasAiActiveBeforeGit;
                      provider.setGitViewActive(false);
                      if (wasAi) provider.setAiViewActive(true);
                    } else {
                      provider.setGitViewActive(true);
                    }
                  }
                },
              ),
              _ToolButton(
                icon: Icons.auto_awesome_outlined,
                isActive: provider.isAiViewActive && provider.isRightSidebarOpen,
                onTap: () {
                  if (!provider.isRightSidebarOpen) {
                    provider.toggleRightSidebar();
                    if (!provider.isAiViewActive) {
                      provider.setAiViewActive(true);
                    }
                  } else {
                    if (provider.isAiViewActive) {
                      provider.setAiViewActive(false);
                    } else {
                      provider.setAiViewActive(true);
                    }
                  }
                },
              ),
              _ToolButton(
                icon: Icons.swap_horiz_rounded,
                onTap: () async {
                  try {
                    await provider.bridge.openPingWindow();
                    provider.showToast('Ping window opened', type: 'success');
                  } catch (e) {
                    provider.showToast('Failed to open Ping window', type: 'error');
                  }
                },
              ),
              _ToolButton(
                icon: Icons.language_outlined,
                onTap: () async {
                  try {
                    await provider.bridge.openBrowserWindow();
                    provider.showToast('Browser opened', type: 'success');
                  } catch (e) {
                    provider.showToast('Failed to open Browser', type: 'error');
                  }
                },
              ),
              _ToolButton(
                icon: Icons.dns_outlined,
                onTap: () => provider.handleFileOpen('__environment__'),
              ),
              _ToolButton(
                icon: Icons.settings_outlined,
                onTap: () async {
                  try {
                    await provider.bridge.openAdminWindow();
                    provider.showToast('Admin panel opened', type: 'success');
                  } catch (e) {
                    provider.showToast('Failed to open Admin panel', type: 'error');
                  }
                },
              ),
              _ToolButton(
                icon: Icons.help_outline,
                onTap: () {},
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _NavTab extends StatelessWidget {
  final IconData icon;
  final VoidCallback onTap;
  final bool isActive;
  final double size;
  final bool flip;

  const _NavTab({
    required this.icon,
    required this.onTap,
    this.isActive = false,
    this.size = 16,
    this.flip = false,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8),
        height: 32,
        alignment: Alignment.center,
        child: Icon(
          icon,
          size: size,
          color: isActive
              ? AppTheme.accentColor(context)
              : AppTheme.textSecondary(context),
        ),
      ),
    );
  }
}

class _ToolButton extends StatelessWidget {
  final IconData icon;
  final VoidCallback onTap;
  final bool isActive;

  const _ToolButton({
    required this.icon,
    required this.onTap,
    this.isActive = false,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 32,
        height: 32,
        alignment: Alignment.center,
        decoration: isActive
            ? BoxDecoration(
                color: AppTheme.hoverColor(context),
              )
            : null,
        child: Icon(
          icon,
          size: 14,
          color: isActive
              ? AppTheme.accentColor(context)
              : AppTheme.textSecondary(context),
        ),
      ),
    );
  }
}
