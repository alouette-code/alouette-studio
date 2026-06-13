import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

class ConfirmDialogOverlay extends StatelessWidget {
  const ConfirmDialogOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final dialog = provider.confirmModal;
        if (dialog == null) return const SizedBox.shrink();

        return GestureDetector(
          onTap: () {
            dialog.onCancel?.call();
            provider.dismissConfirm();
          },
          child: Container(
            color: Colors.black.withOpacity(0.5),
            alignment: Alignment.center,
            child: GestureDetector(
              onTap: () {},
              child: Container(
                width: 380,
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
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Row(
                      children: [
                        Icon(
                          Icons.help_outline,
                          size: 18,
                          color: AppColors.darkWarning,
                        ),
                        const SizedBox(width: 8),
                        Text(
                          'Confirm Action',
                          style: TextStyle(
                            color: AppTheme.textPrimary(context),
                            fontWeight: FontWeight.w600,
                            fontSize: 14,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                    Text(
                      dialog.message,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 13,
                      ),
                    ),
                    const SizedBox(height: 20),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        TextButton(
                          onPressed: () {
                            dialog.onCancel?.call();
                            provider.dismissConfirm();
                          },
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
                            dialog.onConfirm();
                            provider.dismissConfirm();
                          },
                          style: ElevatedButton.styleFrom(
                            backgroundColor: AppColors.darkAccent,
                            padding: const EdgeInsets.symmetric(
                              horizontal: 16,
                              vertical: 8,
                            ),
                          ),
                          child: const Text(
                            'Confirm',
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
