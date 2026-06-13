import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';
import '../models/project.dart';

/// Trang Welcome / Landing dịch từ WelcomePage.tsx.
///
/// Giao diện phẳng cứng cáp (flat monochrome) kết hợp với ô nhập chat trợ lý AI bo tròn nổi bật.
class WelcomePage extends StatefulWidget {
  const WelcomePage({super.key});

  @override
  State<WelcomePage> createState() => _WelcomePageState();
}

class _WelcomePageState extends State<WelcomePage> {
  final TextEditingController _chatController = TextEditingController();
  final ScrollController _chatScrollController = ScrollController();

  bool _isChatting = false;
  bool _isTyping = false;
  final List<_ChatMessage> _messages = [];

  String _status = 'idle'; // 'idle', 'starting', 'running'
  String _statusMessage = 'Sẵn sàng';
  Timer? _streamingTimer;

  List<String> _recentFolders = [];
  List<String> _recentFiles = [];

  @override
  void initState() {
    super.initState();
    _loadRecents();
  }

  @override
  void dispose() {
    _streamingTimer?.cancel();
    _chatController.dispose();
    _chatScrollController.dispose();
    super.dispose();
  }

  Future<void> _loadRecents() async {
    try {
      final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
      if (home != null) {
        final file = File('$home/.alouette_recents.json');
        if (await file.exists()) {
          final content = await file.readAsString();
          final data = jsonDecode(content);
          if (mounted) {
            setState(() {
              _recentFolders = List<String>.from(data['recent_folders'] ?? []);
              _recentFiles = List<String>.from(data['recent_files'] ?? []);
            });
          }
        }
      }
    } catch (e) {
      debugPrint('Không thể tải lịch sử gần đây: $e');
    }
  }

  Future<void> _saveRecents() async {
    try {
      final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
      if (home != null) {
        final file = File('$home/.alouette_recents.json');
        final data = {
          'recent_folders': _recentFolders,
          'recent_files': _recentFiles,
        };
        await file.writeAsString(jsonEncode(data));
      }
    } catch (e) {
      debugPrint('Không thể lưu lịch sử gần đây: $e');
    }
  }

  void _addRecentFolder(String path) {
    if (path.isEmpty) return;
    setState(() {
      _recentFolders.remove(path);
      _recentFolders.insert(0, path);
      if (_recentFolders.length > 5) {
        _recentFolders = _recentFolders.sublist(0, 5);
      }
    });
    _saveRecents();
  }

  void _addRecentFile(String path) {
    if (path.isEmpty) return;
    setState(() {
      _recentFiles.remove(path);
      _recentFiles.insert(0, path);
      if (_recentFiles.length > 5) {
        _recentFiles = _recentFiles.sublist(0, 5);
      }
    });
    _saveRecents();
  }

  String _getBaseName(String path) {
    final normalized = path.replaceAll('\\', '/');
    final lastSlash = normalized.lastIndexOf('/');
    return lastSlash != -1 ? normalized.substring(lastSlash + 1) : path;
  }

  void _handleSendChat() {
    final text = _chatController.text.trim();
    if (text.isEmpty || _isTyping) return;

    setState(() {
      _isChatting = true;
      _isTyping = true;
      _status = 'starting';
      _statusMessage = 'Đang khởi động local server...';
      _messages.add(_ChatMessage(role: 'user', content: text));
      _messages.add(_ChatMessage(role: 'assistant', content: ''));
    });
    _chatController.clear();
    _scrollToBottom();

    // Giả lập tiến trình khởi động server trong 1 giây
    Future.delayed(const Duration(seconds: 1), () {
      if (!mounted || !_isTyping) return;

      setState(() {
        _status = 'running';
        _statusMessage = 'Mô hình local đang chạy';
      });

      // Bắt đầu truyền dữ liệu phản hồi (streaming) sau 500ms
      Future.delayed(const Duration(milliseconds: 500), () {
        if (!mounted || !_isTyping) return;

        final answer = _generateMockResponse(text);
        int index = 0;

        _streamingTimer = Timer.periodic(const Duration(milliseconds: 25), (timer) {
          if (!mounted || !_isTyping) {
            timer.cancel();
            return;
          }

          if (index < answer.length) {
            setState(() {
              if (_messages.isNotEmpty && _messages.last.role == 'assistant') {
                final currentContent = _messages.last.content;
                _messages[_messages.length - 1] = _ChatMessage(
                  role: 'assistant',
                  content: currentContent + answer[index],
                );
              }
            });
            index++;
            _scrollToBottom();
          } else {
            timer.cancel();
            setState(() {
              _isTyping = false;
            });
          }
        });
      });
    });
  }

  void _handleStopChat() {
    _streamingTimer?.cancel();
    setState(() {
      _isTyping = false;
      _status = 'idle';
      _statusMessage = 'Đã dừng phản hồi';
      if (_messages.isNotEmpty && _messages.last.role == 'assistant') {
        final content = _messages.last.content;
        _messages[_messages.length - 1] = _ChatMessage(
          role: 'assistant',
          content: content + '\n\n*Đã dừng phản hồi bởi người dùng.*',
        );
      }
    });
    context.read<AppProvider>().showToast('Đã dừng phản hồi', type: 'info');
  }

  String _generateMockResponse(String userPrompt) {
    final lower = userPrompt.toLowerCase();
    if (lower.contains('dự án') || lower.contains('project')) {
      return 'Dưới đây là một số thông tin về quản lý dự án trong **Alouette Studio**:\n\n'
          '- **Dự án hoạt động**: Hiển thị danh sách các dự án đang chạy hoặc dừng.\n'
          '- **Chạy/Dừng**: Bạn có thể nhấn nút `Chạy` để khởi chạy quy trình dự án hoặc `Dừng` để chấm dứt.\n'
          '- **Dự án mẫu**: Cung cấp cấu hình mẫu dựng sẵn để thử nghiệm nhanh.\n\n'
          'Để tạo dự án mới, hãy sử dụng nút **Dự án mới** ở trên hoặc nhập đường dẫn cục bộ.';
    } else if (lower.contains('git')) {
      return 'Hệ thống hỗ trợ quản lý **Git** trực quan thông qua thanh trạng thái:\n\n'
          '- Xem thay đổi của các tệp tin (Staged / Unstaged).\n'
          '- Thực hiện stage (`+`), unstage (`-`) hoặc khôi phục tệp tin.\n'
          '- Commit với tin nhắn, thực hiện Push / Pull trực tiếp tới remote repository.\n\n'
          'Bạn có thể mở Git Panel từ nút góc dưới bên phải màn hình.';
    } else if (lower.contains('sqlite') || lower.contains('database') || lower.contains('cơ sở dữ liệu')) {
      return 'Trình duyệt cơ sở dữ liệu **SQLite** tích hợp sẵn cho phép:\n\n'
          '- Xem danh sách tất cả các bảng trong database.\n'
          '- Thêm cột mới, thêm dòng hoặc xóa dòng.\n'
          '- Chỉnh sửa trực tiếp từng ô dữ liệu trên bảng.\n\n'
          'Mở bất kỳ tệp tin `.db` hoặc `.sqlite` trong File Explorer để kích hoạt SQLite Editor.';
    } else {
      return 'Chào mừng bạn đến với **Alouette Studio**!\n\n'
          'Tôi là trợ lý trí tuệ nhân tạo chạy trên mô hình ngôn ngữ local (ví dụ: `Phi-3-mini`).\n\n'
          'Tôi có thể giúp bạn:\n'
          '1. Quản lý các quy trình dịch vụ và giám sát CPU/RAM.\n'
          '2. Hướng dẫn thiết lập sandbox cấu hình bảo mật.\n'
          '3. Hỗ trợ thao tác Git, API Client (MiniPostman), SQLite.\n\n'
          'Hãy thử hỏi tôi về **dự án**, **git** hoặc **cơ sở dữ liệu** để xem thêm chi tiết.';
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_chatScrollController.hasClients) {
        _chatScrollController.animateTo(
          _chatScrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Widget _buildStatusDot(String type) {
    Color color;
    switch (type) {
      case 'running':
        color = AppColors.darkSuccess;
        break;
      case 'starting':
      case 'setup':
        color = AppColors.darkWarning;
        break;
      case 'crashing':
      case 'fatal':
      case 'terminated':
        color = AppColors.darkDanger;
        break;
      default:
        color = AppColors.darkTextSecondary;
        break;
    }
    return Container(
      width: 6,
      height: 6,
      decoration: BoxDecoration(
        color: color,
        shape: BoxShape.circle,
        boxShadow: type == 'running' || type == 'starting'
            ? [
                BoxShadow(
                  color: color.withValues(alpha: 0.6),
                  blurRadius: 6,
                  spreadRadius: 1,
                ),
              ]
            : null,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final bgColor = isDark ? const Color(0xFF0D0D14) : const Color(0xFFF5F5F8);
    final surfaceColor = isDark ? const Color(0xFF14141E) : Colors.white;
    final borderColor = isDark ? const Color(0xFF2A2A3A) : const Color(0xFFE0E0E8);

    return Scaffold(
      backgroundColor: bgColor,
      body: SizedBox.expand(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 48),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 800),
              child: Consumer<AppProvider>(
                builder: (context, provider, _) {
                  return Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      // ── HERO ──
                      Text(
                        'Alouette Studio',
                        style: TextStyle(
                          fontSize: 32,
                          fontWeight: FontWeight.w700,
                          letterSpacing: -0.5,
                          color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                        ),
                      ),
                      const SizedBox(height: 24),

                      // ── AI RESPONSE AREA ──
                      if (_isChatting) _buildChatResponseArea(isDark, surfaceColor, borderColor),

                      // ── CHAT INPUT ──
                      _buildChatInputWrapper(isDark),
                      const SizedBox(height: 20),

                      // ── QUICK ACTIONS ──
                      _buildQuickActions(context, isDark, surfaceColor, borderColor, provider),
                      const SizedBox(height: 28),

                      // ── MAIN BODY GRID ──
                      LayoutBuilder(
                        builder: (context, constraints) {
                          if (constraints.maxWidth < 650) {
                            return Column(
                              crossAxisAlignment: CrossAxisAlignment.stretch,
                              children: [
                                _buildProjectsSection(context, isDark, surfaceColor, borderColor, provider, false),
                                const SizedBox(height: 20),
                                _buildRightSidebar(context, isDark, surfaceColor, borderColor, provider),
                              ],
                            );
                          } else {
                            return IntrinsicHeight(
                              child: Row(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Expanded(
                                    flex: 14,
                                    child: _buildProjectsSection(context, isDark, surfaceColor, borderColor, provider, true),
                                  ),
                                  const SizedBox(width: 20),
                                  Expanded(
                                    flex: 10,
                                    child: _buildRightSidebar(context, isDark, surfaceColor, borderColor, provider),
                                  ),
                                ],
                              ),
                            );
                          }
                        },
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildChatResponseArea(bool isDark, Color surfaceColor, Color borderColor) {
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 20),
      decoration: BoxDecoration(
        color: isDark ? const Color(0xFF14141E) : const Color(0xFFFAFAFC),
        borderRadius: BorderRadius.zero,
        border: Border.all(color: borderColor),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: isDark ? 0.2 : 0.05),
            blurRadius: 10,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          // Header Status
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: borderColor)),
            ),
            child: Row(
              children: [
                _buildStatusDot(_status),
                const SizedBox(width: 8),
                Text(
                  _statusMessage,
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w500,
                    color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                  ),
                ),
              ],
            ),
          ),
          // Scrollable messages
          ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 280),
            child: ShaderMask(
              shaderCallback: (Rect rect) {
                return const LinearGradient(
                  begin: Alignment.topCenter,
                  end: Alignment.bottomCenter,
                  colors: [Colors.transparent, Colors.black, Colors.black],
                  stops: [0.0, 0.1, 1.0],
                ).createShader(rect);
              },
              blendMode: BlendMode.dstIn,
              child: ListView.builder(
                controller: _chatScrollController,
                padding: const EdgeInsets.fromLTRB(12, 16, 12, 16),
                shrinkWrap: true,
                itemCount: _messages.length,
                itemBuilder: (context, index) {
                  final msg = _messages[index];
                  final isUser = msg.role == 'user';
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Align(
                      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
                      child: Container(
                        constraints: BoxConstraints(
                          maxWidth: MediaQuery.of(context).size.width * 0.65,
                        ),
                        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                        decoration: BoxDecoration(
                          gradient: isUser
                              ? const LinearGradient(
                                  colors: [Color(0xFF3A86FF), Color(0xFF6366F1)],
                                  begin: Alignment.topLeft,
                                  end: Alignment.bottomRight,
                                )
                              : null,
                          color: isUser
                              ? null
                              : (isDark ? const Color(0xFF20202A) : const Color(0xFFFFFFFF)),
                          border: isUser
                              ? null
                              : Border.all(color: const Color(0xFF8B5CF6).withValues(alpha: 0.15)),
                          borderRadius: isUser
                              ? const BorderRadius.only(
                                  topLeft: Radius.circular(14),
                                  topRight: Radius.circular(14),
                                  bottomLeft: Radius.circular(14),
                                  bottomRight: Radius.circular(4),
                                )
                              : const BorderRadius.only(
                                  topLeft: Radius.circular(14),
                                  topRight: Radius.circular(14),
                                  bottomLeft: Radius.circular(4),
                                  bottomRight: Radius.circular(14),
                                ),
                          boxShadow: isUser
                              ? [
                                  BoxShadow(
                                    color: const Color(0xFF6366F1).withValues(alpha: 0.25),
                                    blurRadius: 8,
                                    offset: const Offset(0, 2),
                                  ),
                                ]
                              : [
                                  BoxShadow(
                                    color: Colors.black.withValues(alpha: isDark ? 0.08 : 0.02),
                                    blurRadius: 4,
                                    offset: const Offset(0, 1),
                                  ),
                                ],
                        ),
                        child: !isUser && msg.content.isEmpty
                            ? Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  _buildThinkingDots(),
                                  const SizedBox(width: 8),
                                  Text(
                                    'Đang suy nghĩ',
                                    style: TextStyle(
                                      fontSize: 11,
                                      fontWeight: FontWeight.w500,
                                      color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                                    ),
                                  ),
                                ],
                              )
                            : _MarkdownText(text: msg.content, isDark: isDark),
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildThinkingDots() {
    return const _ThinkingIndicator();
  }

  Widget _buildChatInputWrapper(bool isDark) {
    return SizedBox(
      width: double.infinity,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: _WelcomeChatInputBox(
              controller: _chatController,
              isDark: isDark,
              isTyping: _isTyping,
              onSubmitted: _handleSendChat,
            ),
          ),
          const SizedBox(width: 12),
          _WelcomeChatSendBtn(
            isTyping: _isTyping,
            isEnabled: _chatController.text.trim().isNotEmpty,
            onTap: _isTyping ? _handleStopChat : _handleSendChat,
          ),
        ],
      ),
    );
  }

  Widget _buildQuickActions(
      BuildContext context, bool isDark, Color surfaceColor, Color borderColor, AppProvider provider) {
    return Wrap(
      spacing: 10,
      runSpacing: 10,
      alignment: WrapAlignment.center,
      children: [
        _QuickActionBtn(
          icon: Icons.add_rounded,
          label: 'Dự án mới',
          isDark: isDark,
          surfaceColor: surfaceColor,
          borderColor: borderColor,
          onTap: () {
            provider.showToast('Tạo dự án mới', type: 'info');
          },
        ),
        _QuickActionBtn(
          icon: Icons.folder_open_rounded,
          label: 'Mở thư mục',
          isDark: isDark,
          surfaceColor: surfaceColor,
          borderColor: borderColor,
          onTap: () async {
            final path = await provider.bridge.openFolderDialog();
            if (path != null && path.isNotEmpty) {
              _addRecentFolder(path);
              provider.showToast('Đã mở thư mục: $path', type: 'success');
            }
          },
        ),
        _QuickActionBtn(
          icon: Icons.description_outlined,
          label: 'Mở tệp',
          isDark: isDark,
          surfaceColor: surfaceColor,
          borderColor: borderColor,
          onTap: () async {
            final path = await provider.bridge.openFileDialog();
            if (path != null && path.isNotEmpty) {
              _addRecentFile(path);
              provider.showToast('Đã mở tệp: $path', type: 'success');
            }
          },
        ),
        _QuickActionBtn(
          icon: Icons.storage_rounded,
          label: 'Dự án Mẫu',
          isDark: isDark,
          surfaceColor: surfaceColor,
          borderColor: borderColor,
          onTap: () {
            provider.showToast('Đang tải dự án mẫu demo...', type: 'info');
          },
        ),
      ],
    );
  }

  Widget _buildProjectsSection(
      BuildContext context, bool isDark, Color surfaceColor, Color borderColor, AppProvider provider, bool isRowMode) {
    final projects = provider.projects;
    final projectStates = provider.projectStates;

    Widget content;
    if (projects.isEmpty) {
      content = Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.terminal_rounded,
                  size: 24,
                  color: isDark
                      ? AppColors.darkTextSecondary.withValues(alpha: 0.4)
                      : AppColors.lightTextSecondary.withValues(alpha: 0.4)),
              const SizedBox(height: 8),
              Text(
                'Chưa có dự án nào được đăng ký',
                style: TextStyle(
                  fontSize: 12,
                  color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                ),
              ),
              const SizedBox(height: 12),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: Colors.transparent,
                  surfaceTintColor: Colors.transparent,
                  shadowColor: Colors.transparent,
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.zero,
                    side: BorderSide(color: borderColor),
                  ),
                ),
                onPressed: () => provider.showToast('Đang tải dự án mẫu demo...', type: 'info'),
                child: Text(
                  'Tải dự án mẫu demo',
                  style: TextStyle(
                    fontSize: 10.5,
                    color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    } else {
      content = ListView.separated(
        shrinkWrap: true,
        physics: const NeverScrollableScrollPhysics(),
        itemCount: projects.length,
        separatorBuilder: (_, __) => const SizedBox(height: 6),
        itemBuilder: (context, index) {
          final proj = projects[index];
          final state = projectStates[proj.id] ?? ProcessState(type: ProcessStateType.stopped);
          final isRunning = state.type == ProcessStateType.running || state.type == ProcessStateType.setup;

          return _ProjectRowCard(
            proj: proj,
            state: state,
            isRunning: isRunning,
            isDark: isDark,
            borderColor: borderColor,
            onTap: () => provider.activeProjectId = proj.id,
            onStart: () => provider.handleStartProject(proj.id),
            onStop: () => provider.handleStopProject(proj.id),
          );
        },
      );
    }

    return Container(
      decoration: BoxDecoration(
        color: surfaceColor,
        borderRadius: BorderRadius.zero,
        border: Border.all(color: borderColor),
      ),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Row(
            children: [
              Icon(Icons.grid_view_rounded, size: 14, color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
              const SizedBox(width: 6),
              Text(
                'Dự án hoạt động',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.w600,
                  color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                ),
              ),
              const Spacer(),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                decoration: BoxDecoration(
                  color: isDark ? const Color(0xFF222230) : const Color(0xFFE8E8EE),
                  border: Border.all(color: borderColor),
                ),
                child: Text(
                  '${projects.length}',
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.w600,
                    color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Divider(color: borderColor, height: 1),
          const SizedBox(height: 12),

          // Body
          isRowMode ? Expanded(child: content) : content,
        ],
      ),
    );
  }

  Widget _buildRightSidebar(
      BuildContext context, bool isDark, Color surfaceColor, Color borderColor, AppProvider provider) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // ── Mục gần đây ──
        Container(
          decoration: BoxDecoration(
            color: surfaceColor,
            borderRadius: BorderRadius.zero,
            border: Border.all(color: borderColor),
          ),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                children: [
                  Icon(Icons.history_rounded, size: 14, color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
                  const SizedBox(width: 6),
                  Text(
                    'Mục gần đây',
                    style: TextStyle(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w600,
                      color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              Divider(color: borderColor, height: 1),
              const SizedBox(height: 10),
              if (_recentFolders.isEmpty && _recentFiles.isEmpty)
                Text(
                  'Không có lịch sử mở gần đây',
                  style: TextStyle(
                    fontSize: 11,
                    color: isDark
                        ? AppColors.darkTextSecondary.withValues(alpha: 0.6)
                        : AppColors.lightTextSecondary.withValues(alpha: 0.6),
                  ),
                )
              else ...[
                if (_recentFolders.isNotEmpty) ...[
                  Text(
                    'Thư mục',
                    style: TextStyle(
                      fontSize: 10.5,
                      fontWeight: FontWeight.w600,
                      color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                    ),
                  ),
                  const SizedBox(height: 4),
                  ..._recentFolders.map((folder) => _RecentItem(
                        label: _getBaseName(folder),
                        fullPath: folder,
                        isDark: isDark,
                        onTap: () {
                          provider.showToast('Đang mở thư mục: $folder', type: 'info');
                          provider.handleFileOpen(folder);
                        },
                      )),
                  const SizedBox(height: 8),
                ],
                if (_recentFiles.isNotEmpty) ...[
                  Text(
                    'Tệp tin',
                    style: TextStyle(
                      fontSize: 10.5,
                      fontWeight: FontWeight.w600,
                      color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                    ),
                  ),
                  const SizedBox(height: 4),
                  ..._recentFiles.map((file) => _RecentItem(
                        label: _getBaseName(file),
                        fullPath: file,
                        isDark: isDark,
                        onTap: () {
                          provider.showToast('Đang mở tệp: $file', type: 'info');
                          provider.handleFileOpen(file);
                        },
                      )),
                ],
              ],
            ],
          ),
        ),
        const SizedBox(height: 16),

        // ── Công cụ khác ──
        Container(
          decoration: BoxDecoration(
            color: surfaceColor,
            borderRadius: BorderRadius.zero,
            border: Border.all(color: borderColor),
          ),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                children: [
                  Icon(Icons.dns_outlined, size: 14, color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
                  const SizedBox(width: 6),
                  Text(
                    'Công cụ khác',
                    style: TextStyle(
                      fontSize: 12.5,
                      fontWeight: FontWeight.w600,
                      color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              Divider(color: borderColor, height: 1),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: _FlatToolBtn(
                      icon: Icons.settings_rounded,
                      label: 'Admin',
                      isDark: isDark,
                      borderColor: borderColor,
                      onTap: () => provider.bridge.openAdminWindow(),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: _FlatToolBtn(
                      icon: Icons.swap_horiz_rounded,
                      label: 'Postman API',
                      isDark: isDark,
                      borderColor: borderColor,
                      onTap: () => provider.bridge.openPingWindow(),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

// ── Hỗ trợ Render Markdown thủ công để tránh xung đột thư viện ──

class _MarkdownText extends StatelessWidget {
  final String text;
  final bool isDark;

  const _MarkdownText({required this.text, required this.isDark});

  @override
  Widget build(BuildContext context) {
    final blocks = _parseBlocks(text);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: blocks.map((block) => _buildBlockWidget(block, context)).toList(),
    );
  }

  List<_MarkdownBlock> _parseBlocks(String text) {
    final List<_MarkdownBlock> blocks = [];
    final lines = text.split('\n');
    bool inCodeBlock = false;
    List<String> codeBlockLines = [];

    for (var line in lines) {
      if (line.trim().startsWith('```')) {
        if (inCodeBlock) {
          blocks.add(_MarkdownBlock(type: _BlockType.code, content: codeBlockLines.join('\n')));
          codeBlockLines = [];
          inCodeBlock = false;
        } else {
          inCodeBlock = true;
        }
      } else {
        if (inCodeBlock) {
          codeBlockLines.add(line);
        } else {
          final trimmed = line.trim();
          if (trimmed.isEmpty) {
            blocks.add(_MarkdownBlock(type: _BlockType.spacing, content: ''));
          } else if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
            blocks.add(_MarkdownBlock(type: _BlockType.bullet, content: trimmed.substring(2)));
          } else {
            blocks.add(_MarkdownBlock(type: _BlockType.paragraph, content: line));
          }
        }
      }
    }

    if (inCodeBlock && codeBlockLines.isNotEmpty) {
      blocks.add(_MarkdownBlock(type: _BlockType.code, content: codeBlockLines.join('\n')));
    }

    return blocks;
  }

  Widget _buildBlockWidget(_MarkdownBlock block, BuildContext context) {
    switch (block.type) {
      case _BlockType.code:
        return Container(
          width: double.infinity,
          margin: const EdgeInsets.symmetric(vertical: 6),
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: isDark ? const Color(0xFF0A0A10) : const Color(0xFFF0F0F4),
            borderRadius: BorderRadius.zero,
            border: Border.all(color: isDark ? const Color(0xFF252535) : const Color(0xFFE0E0E8)),
          ),
          child: Text(
            block.content,
            style: TextStyle(
              fontFamily: 'monospace',
              fontSize: 11,
              color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
            ),
          ),
        );
      case _BlockType.bullet:
        return Padding(
          padding: const EdgeInsets.only(left: 8, bottom: 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('• ',
                  style: TextStyle(
                      color: isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary, fontSize: 12)),
              Expanded(
                child: _buildRichText(block.content, context),
              ),
            ],
          ),
        );
      case _BlockType.spacing:
        return const SizedBox(height: 6);
      case _BlockType.paragraph:
        return Padding(
          padding: const EdgeInsets.only(bottom: 6),
          child: _buildRichText(block.content, context),
        );
    }
  }

  Widget _buildRichText(String content, BuildContext context) {
    final List<InlineSpan> spans = [];
    final regExp = RegExp(r'(\*\*.*?\*\*|`.*?`)');
    final matches = regExp.allMatches(content);

    int lastMatchEnd = 0;
    for (var match in matches) {
      if (match.start > lastMatchEnd) {
        spans.add(TextSpan(
          text: content.substring(lastMatchEnd, match.start),
          style: TextStyle(
            color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
            fontSize: 12.5,
            height: 1.4,
          ),
        ));
      }

      final matchedText = match.group(0)!;
      if (matchedText.startsWith('**') && matchedText.endsWith('**')) {
        spans.add(TextSpan(
          text: matchedText.substring(2, matchedText.length - 2),
          style: TextStyle(
            color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
            fontWeight: FontWeight.bold,
            fontSize: 12.5,
            height: 1.4,
          ),
        ));
      } else if (matchedText.startsWith('`') && matchedText.endsWith('`')) {
        spans.add(WidgetSpan(
          alignment: PlaceholderAlignment.middle,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
            decoration: BoxDecoration(
              color: isDark ? Colors.black.withValues(alpha: 0.3) : Colors.black.withValues(alpha: 0.05),
              borderRadius: BorderRadius.zero,
            ),
            child: Text(
              matchedText.substring(1, matchedText.length - 1),
              style: TextStyle(
                fontFamily: 'monospace',
                fontSize: 11,
                color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
              ),
            ),
          ),
        ));
      }
      lastMatchEnd = match.end;
    }

    if (lastMatchEnd < content.length) {
      spans.add(TextSpan(
        text: content.substring(lastMatchEnd),
        style: TextStyle(
          color: isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
          fontSize: 12.5,
          height: 1.4,
        ),
      ));
    }

    return Text.rich(
      TextSpan(children: spans),
    );
  }
}

enum _BlockType { paragraph, code, bullet, spacing }

class _MarkdownBlock {
  final _BlockType type;
  final String content;
  _MarkdownBlock({required this.type, required this.content});
}

// ── Widget Hộp Nhập Trợ Lý AI viền Gradient bo tròn (Pill shape) như React ──

class _WelcomeChatInputBox extends StatefulWidget {
  final TextEditingController controller;
  final bool isDark;
  final bool isTyping;
  final VoidCallback onSubmitted;

  const _WelcomeChatInputBox({
    required this.controller,
    required this.isDark,
    required this.isTyping,
    required this.onSubmitted,
  });

  @override
  State<_WelcomeChatInputBox> createState() => _WelcomeChatInputBoxState();
}

class _WelcomeChatInputBoxState extends State<_WelcomeChatInputBox> {
  bool _isHovered = false;
  bool _isFocused = false;
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _focusNode.addListener(() {
      if (mounted) {
        setState(() {
          _isFocused = _focusNode.hasFocus;
        });
      }
    });
  }

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final innerBgColor = widget.isDark
        ? (_isHovered || _isFocused ? const Color(0xFF14141E) : const Color(0xFF111118))
        : (_isHovered || _isFocused ? const Color(0xFFF3F3FA) : const Color(0xFFFFFFFF));

    final gradientColors = _isFocused
        ? const [Color(0xFF6366F1), Color(0xFFEC4899)]
        : (_isHovered
            ? const [Color(0xFF8B5CF6), Color(0xFFEC4899)]
            : const [Color(0xFF6366F1), Color(0xFFEC4899)]);

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(30), // Bo tròn (Pill-shaped) y hệt React
          gradient: LinearGradient(
            colors: gradientColors,
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
          boxShadow: [
            BoxShadow(
              color: const Color(0xFF6366F1).withValues(alpha: _isFocused ? 0.25 : (_isHovered ? 0.18 : 0.12)),
              blurRadius: _isFocused ? 32 : (_isHovered ? 24 : 16),
              offset: const Offset(0, 6),
              spreadRadius: -4,
            ),
            BoxShadow(
              color: const Color(0xFFEC4899).withValues(alpha: _isFocused ? 0.15 : (_isHovered ? 0.10 : 0.05)),
              blurRadius: _isFocused ? 16 : (_isHovered ? 12 : 8),
              offset: Offset.zero,
            ),
          ],
        ),
        padding: const EdgeInsets.all(2), // Viền gradient
        child: Container(
          height: 52,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(28), // Trùng với viền ngoài
            color: innerBgColor,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 20),
          alignment: Alignment.center,
          child: TextField(
            controller: widget.controller,
            focusNode: _focusNode,
            enabled: !widget.isTyping,
            style: TextStyle(
              fontSize: 14.5,
              fontWeight: FontWeight.w500,
              color: widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
            ),
            decoration: InputDecoration(
              hintText: 'Nhập câu hỏi hoặc yêu cầu gửi tới trợ lý AI...',
              hintStyle: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w400,
                color: widget.isDark
                    ? AppColors.darkTextSecondary.withValues(alpha: 0.7)
                    : AppColors.lightTextSecondary.withValues(alpha: 0.7),
              ),
              border: InputBorder.none,
              isDense: true,
              contentPadding: const EdgeInsets.symmetric(vertical: 10),
            ),
            onSubmitted: (_) => widget.onSubmitted(),
          ),
        ),
      ),
    );
  }
}

// ── Widget Nút Gửi / Dừng AI Chat Tròn Có Hiệu Ứng Phóng To như React ──

class _WelcomeChatSendBtn extends StatefulWidget {
  final bool isTyping;
  final bool isEnabled;
  final VoidCallback onTap;

  const _WelcomeChatSendBtn({
    required this.isTyping,
    required this.isEnabled,
    required this.onTap,
  });

  @override
  State<_WelcomeChatSendBtn> createState() => _WelcomeChatSendBtnState();
}

class _WelcomeChatSendBtnState extends State<_WelcomeChatSendBtn> {
  bool _isHovered = false;
  bool _isActive = false;

  @override
  Widget build(BuildContext context) {
    final isClickable = widget.isTyping || widget.isEnabled;

    // Gradient y hệt React: 
    // - Khi đang gõ/chạy AI (isTyping): đỏ -> đỏ đậm (dừng)
    // - Khi rảnh & hover: #8b5cf6 -> #ec4899 -> #3b82f6
    // - Khi bình thường (bao gồm cả disabled): #6366f1 -> #a78bfa -> #ec4899
    final List<Color> gradientColors;
    if (widget.isTyping) {
      gradientColors = const [Color(0xFFEF4444), Color(0xFFDC2626)];
    } else if (isClickable && _isHovered) {
      gradientColors = const [Color(0xFF8B5CF6), Color(0xFFEC4899), Color(0xFF3B82F6)];
    } else {
      gradientColors = const [Color(0xFF6366F1), Color(0xFFA78BFA), Color(0xFFEC4899)];
    }

    final scale = isClickable ? (_isActive ? 0.95 : (_isHovered ? 1.08 : 1.0)) : 1.0;
    final rotation = isClickable && _isHovered ? -0.174 : 0.0; // -10 độ theo radian

    return MouseRegion(
      cursor: isClickable ? SystemMouseCursors.click : SystemMouseCursors.basic,
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTapDown: (_) => setState(() => _isActive = true),
        onTapUp: (_) => setState(() => _isActive = false),
        onTapCancel: () => setState(() => _isActive = false),
        onTap: isClickable ? widget.onTap : null,
        child: Transform.rotate(
          angle: rotation,
          child: Transform.scale(
            scale: scale,
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 150),
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                shape: BoxShape.circle, // Hình tròn hoàn hảo như React
                gradient: LinearGradient(
                  colors: gradientColors,
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                boxShadow: [
                  BoxShadow(
                    color: (widget.isTyping ? const Color(0xFFEF4444) : const Color(0xFFEC4899))
                        .withValues(alpha: (isClickable && _isHovered) ? 0.5 : 0.25),
                    blurRadius: (isClickable && _isHovered) ? 16 : 10,
                    offset: const Offset(0, 4),
                  ),
                ],
              ),
              child: Center(
                child: widget.isTyping
                    ? const Icon(Icons.stop_rounded, size: 14, color: Colors.white)
                    : const Icon(Icons.arrow_forward_rounded, size: 18, color: Colors.white),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ── Widget Nút Hành Động Nhanh Flat Cứng Cáp ──

class _QuickActionBtn extends StatefulWidget {
  final IconData icon;
  final String label;
  final bool isDark;
  final Color surfaceColor;
  final Color borderColor;
  final VoidCallback onTap;

  const _QuickActionBtn({
    required this.icon,
    required this.label,
    required this.isDark,
    required this.surfaceColor,
    required this.borderColor,
    required this.onTap,
  });

  @override
  State<_QuickActionBtn> createState() => _QuickActionBtnState();
}

class _QuickActionBtnState extends State<_QuickActionBtn> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final finalBgColor = _isHovered
        ? (widget.isDark ? const Color(0xFF222230) : const Color(0xFFEEEEF4))
        : Colors.transparent; // Transparent y hệt React

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
          decoration: BoxDecoration(
            color: finalBgColor,
            borderRadius: BorderRadius.zero,
            border: Border.all(
              color: _isHovered
                  ? (widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary)
                  : widget.borderColor,
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(widget.icon,
                  size: 14,
                  color: _isHovered
                      ? (widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary)
                      : (widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary)),
              const SizedBox(width: 6),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w500,
                  color: _isHovered
                      ? (widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary)
                      : (widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── Widget Dòng Dự Án Hoạt Động Cứng Cáp Cạnh Vuông ──

class _ProjectRowCard extends StatefulWidget {
  final Project proj;
  final ProcessState state;
  final bool isRunning;
  final bool isDark;
  final Color borderColor;
  final VoidCallback onTap;
  final VoidCallback onStart;
  final VoidCallback onStop;

  const _ProjectRowCard({
    required this.proj,
    required this.state,
    required this.isRunning,
    required this.isDark,
    required this.borderColor,
    required this.onTap,
    required this.onStart,
    required this.onStop,
  });

  @override
  State<_ProjectRowCard> createState() => _ProjectRowCardState();
}

class _ProjectRowCardState extends State<_ProjectRowCard> {
  bool _isHovered = false;

  Color _getStatusColor(ProcessStateType type) {
    switch (type) {
      case ProcessStateType.running:
        return AppColors.darkSuccess;
      case ProcessStateType.setup:
        return AppColors.darkWarning;
      case ProcessStateType.crashing:
      case ProcessStateType.fatal:
      case ProcessStateType.terminated:
        return AppColors.darkDanger;
      default:
        return AppColors.darkTextSecondary;
    }
  }

  @override
  Widget build(BuildContext context) {
    final finalBgColor = widget.isDark
        ? (_isHovered ? const Color(0xFF222230) : const Color(0xFF111118)) // background trong suốt hơn
        : (_isHovered ? const Color(0xFFEEEEF4) : const Color(0xFFFAFAFC));

    final finalBorderColor = _isHovered
        ? (widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary)
        : widget.borderColor;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 150),
        decoration: BoxDecoration(
          color: finalBgColor,
          borderRadius: BorderRadius.zero,
          border: Border.all(color: finalBorderColor),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
          child: Row(
            children: [
              // Status dot
              Container(
                width: 6,
                height: 6,
                decoration: BoxDecoration(
                  color: _getStatusColor(widget.state.type),
                  shape: BoxShape.circle,
                  boxShadow: widget.isRunning
                      ? [
                          BoxShadow(
                            color: _getStatusColor(widget.state.type).withValues(alpha: 0.6),
                            blurRadius: 4,
                            spreadRadius: 1,
                          ),
                        ]
                      : null,
                ),
              ),
              const SizedBox(width: 10),
              // Info
              Expanded(
                child: GestureDetector(
                  onTap: widget.onTap,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        children: [
                          Text(
                            widget.proj.name,
                            style: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.w600,
                              color: widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary,
                            ),
                          ),
                          if (widget.proj.toolchain != null) ...[
                            const SizedBox(width: 6),
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 3, vertical: 0.5),
                              decoration: BoxDecoration(
                                color: Colors.transparent,
                                border: Border.all(color: widget.borderColor),
                              ),
                              child: Text(
                                widget.proj.toolchain!,
                                style: TextStyle(
                                  fontSize: 9,
                                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                          ],
                          if (widget.proj.port != null) ...[
                            const SizedBox(width: 4),
                            Container(
                              padding: const EdgeInsets.symmetric(horizontal: 3, vertical: 0.5),
                              decoration: BoxDecoration(
                                color: Colors.transparent,
                                border: Border.all(color: widget.borderColor),
                              ),
                              child: Text(
                                'Port ${widget.proj.port}',
                                style: TextStyle(
                                  fontSize: 9,
                                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ),
                          ],
                        ],
                      ),
                      const SizedBox(height: 2),
                      Text(
                        widget.proj.cwd ?? 'Không có cwd',
                        style: TextStyle(
                          fontSize: 10,
                          color: widget.isDark
                              ? AppColors.darkTextSecondary.withValues(alpha: 0.7)
                              : AppColors.lightTextSecondary.withValues(alpha: 0.7),
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
              ),
              // Actions
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (widget.isRunning)
                    _RowActionButton(
                      label: 'Dừng',
                      icon: Icons.square_rounded,
                      isDark: widget.isDark,
                      borderColor: widget.borderColor,
                      hoverBgColor: const Color(0xFFEF4444).withValues(alpha: 0.08),
                      onTap: widget.onStop,
                    )
                  else
                    _RowActionButton(
                      label: 'Chạy',
                      icon: Icons.play_arrow_rounded,
                      isDark: widget.isDark,
                      borderColor: widget.borderColor,
                      hoverBgColor: const Color(0xFF10B981).withValues(alpha: 0.08),
                      onTap: widget.onStart,
                    ),
                  const SizedBox(width: 4),
                  GestureDetector(
                    onTap: widget.onTap,
                    child: Container(
                      padding: const EdgeInsets.all(5),
                      decoration: BoxDecoration(
                        color: Colors.transparent,
                        border: Border.all(color: widget.borderColor),
                      ),
                      child: Icon(
                        Icons.open_in_new_rounded,
                        size: 10,
                        color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _RowActionButton extends StatefulWidget {
  final String label;
  final IconData icon;
  final bool isDark;
  final Color borderColor;
  final Color hoverBgColor;
  final VoidCallback onTap;

  const _RowActionButton({
    required this.label,
    required this.icon,
    required this.isDark,
    required this.borderColor,
    required this.hoverBgColor,
    required this.onTap,
  });

  @override
  State<_RowActionButton> createState() => _RowActionButtonState();
}

class _RowActionButtonState extends State<_RowActionButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
          decoration: BoxDecoration(
            color: _isHovered ? widget.hoverBgColor : Colors.transparent,
            border: Border.all(color: widget.borderColor),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(widget.icon,
                  size: 10,
                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
              const SizedBox(width: 3),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 10,
                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
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

// ── Widget Dòng Mục Gần Đây Có Gạch Chân Khi Hover ──

class _RecentItem extends StatefulWidget {
  final String label;
  final String fullPath;
  final bool isDark;
  final VoidCallback onTap;

  const _RecentItem({
    required this.label,
    required this.fullPath,
    required this.isDark,
    required this.onTap,
  });

  @override
  State<_RecentItem> createState() => _RecentItemState();
}

class _RecentItemState extends State<_RecentItem> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final textColor = _isHovered
        ? (widget.isDark ? AppColors.darkTextPrimary : AppColors.lightTextPrimary)
        : (widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary);

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.symmetric(vertical: 4),
          color: Colors.transparent,
          child: Text(
            widget.label,
            style: TextStyle(
              fontSize: 11.5,
              color: textColor,
              decoration: _isHovered ? TextDecoration.underline : TextDecoration.none,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ),
    );
  }
}

// ── Widget Nút Công Cụ Phẳng ──

class _FlatToolBtn extends StatefulWidget {
  final IconData icon;
  final String label;
  final bool isDark;
  final Color borderColor;
  final VoidCallback onTap;

  const _FlatToolBtn({
    required this.icon,
    required this.label,
    required this.isDark,
    required this.borderColor,
    required this.onTap,
  });

  @override
  State<_FlatToolBtn> createState() => _FlatToolBtnState();
}

class _FlatToolBtnState extends State<_FlatToolBtn> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final finalBgColor = _isHovered
        ? (widget.isDark ? const Color(0xFF222230) : const Color(0xFFEEEEF4))
        : (widget.isDark ? const Color(0xFF14141E) : const Color(0xFFFAFAFC));

    final finalBorderColor = _isHovered
        ? (widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary)
        : widget.borderColor;

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          padding: const EdgeInsets.symmetric(vertical: 8),
          decoration: BoxDecoration(
            color: finalBgColor,
            borderRadius: BorderRadius.zero,
            border: Border.all(color: finalBorderColor),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(widget.icon,
                  size: 12,
                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary),
              const SizedBox(height: 4),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.w500,
                  color: widget.isDark ? AppColors.darkTextSecondary : AppColors.lightTextSecondary,
                ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ── Widget Trợ Giúp Chat Message ──

class _ChatMessage {
  final String role;
  final String content;
  const _ChatMessage({required this.role, required this.content});
}

// ── Widget Bộ Chỉ Báo Đang Suy Nghĩ (Animation nhảy chấm tròn) ──

class _ThinkingIndicator extends StatefulWidget {
  const _ThinkingIndicator();

  @override
  State<_ThinkingIndicator> createState() => _ThinkingIndicatorState();
}

class _ThinkingIndicatorState extends State<_ThinkingIndicator> with TickerProviderStateMixin {
  late AnimationController _controller1;
  late AnimationController _controller2;
  late AnimationController _controller3;

  @override
  void initState() {
    super.initState();
    _controller1 = AnimationController(vsync: this, duration: const Duration(milliseconds: 800));
    _controller2 = AnimationController(vsync: this, duration: const Duration(milliseconds: 800));
    _controller3 = AnimationController(vsync: this, duration: const Duration(milliseconds: 800));

    _startAnimations();
  }

  void _startAnimations() async {
    if (!mounted) return;
    _controller1.repeat(reverse: true);
    await Future.delayed(const Duration(milliseconds: 160));
    if (!mounted) return;
    _controller2.repeat(reverse: true);
    await Future.delayed(const Duration(milliseconds: 160));
    if (!mounted) return;
    _controller3.repeat(reverse: true);
  }

  @override
  void dispose() {
    _controller1.dispose();
    _controller2.dispose();
    _controller3.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        _buildDot(_controller1, const [Color(0xFF6366F1), Color(0xFF8B5CF6)]),
        const SizedBox(width: 4),
        _buildDot(_controller2, const [Color(0xFF8B5CF6), Color(0xFFA78BFA)]),
        const SizedBox(width: 4),
        _buildDot(_controller3, const [Color(0xFFA78BFA), Color(0xFFEC4899)]),
      ],
    );
  }

  Widget _buildDot(AnimationController controller, List<Color> colors) {
    return ScaleTransition(
      scale: Tween<double>(begin: 0.6, end: 1.2).animate(
        CurvedAnimation(parent: controller, curve: Curves.easeInOut),
      ),
      child: Container(
        width: 6,
        height: 6,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          gradient: LinearGradient(
            colors: colors,
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
        ),
      ),
    );
  }
}
