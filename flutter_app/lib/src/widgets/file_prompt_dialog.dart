import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

class FilePromptOverlay extends StatefulWidget {
  const FilePromptOverlay({super.key});

  @override
  State<FilePromptOverlay> createState() => _FilePromptOverlayState();
}

class _FilePromptOverlayState extends State<FilePromptOverlay> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final prompt = provider.filePrompt;
        if (prompt == null) return const SizedBox.shrink();

        _controller.text = prompt.defaultValue;

        return Container(
          color: Colors.black.withOpacity(0.5),
          alignment: Alignment.center,
          child: GestureDetector(
            onTap: provider.dismissFilePrompt,
            child: Container(
              width: 400,
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                color: Theme.of(context).brightness == Brightness.dark
                    ? const Color(0xFF1E1E2A)
                    : Colors.white,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: AppTheme.borderPrimary(context),
                ),
              ),
              child: GestureDetector(
                onTap: () {},
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      prompt.title,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontWeight: FontWeight.w600,
                        fontSize: 14,
                      ),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: _controller,
                      autofocus: true,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 13,
                      ),
                      decoration: InputDecoration(
                        hintText: prompt.placeholder,
                        hintStyle: TextStyle(
                          color: AppTheme.textSecondary(context),
                          fontSize: 13,
                        ),
                        filled: true,
                        fillColor: Theme.of(context).brightness == Brightness.dark
                            ? const Color(0xFF111118)
                            : const Color(0xFFF5F5F8),
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(6),
                          borderSide: BorderSide(
                            color: AppTheme.borderPrimary(context),
                          ),
                        ),
                        enabledBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(6),
                          borderSide: BorderSide(
                            color: AppTheme.borderPrimary(context),
                          ),
                        ),
                        focusedBorder: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(6),
                          borderSide: BorderSide(
                            color: AppColors.darkAccent,
                          ),
                        ),
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 10,
                        ),
                      ),
                      onSubmitted: (val) {
                        if (val.trim().isNotEmpty) {
                          prompt.onOk(val.trim());
                        }
                      },
                    ),
                    const SizedBox(height: 16),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        TextButton(
                          onPressed: provider.dismissFilePrompt,
                          child: Text(
                            'Cancel',
                            style: TextStyle(
                              color: AppTheme.textSecondary(context),
                              fontSize: 12,
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        ElevatedButton(
                          onPressed: () {
                            final val = _controller.text.trim();
                            if (val.isNotEmpty) {
                              prompt.onOk(val);
                            }
                          },
                          style: ElevatedButton.styleFrom(
                            backgroundColor: AppColors.darkAccent,
                            padding: const EdgeInsets.symmetric(
                              horizontal: 16,
                              vertical: 8,
                            ),
                          ),
                          child: const Text(
                            'OK',
                            style: TextStyle(fontSize: 12, color: Colors.white),
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
