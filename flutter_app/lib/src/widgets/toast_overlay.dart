import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/app_provider.dart';
import '../theme/app_theme.dart';

class ToastOverlay extends StatelessWidget {
  const ToastOverlay({super.key});

  @override
  Widget build(BuildContext context) {
    return Consumer<AppProvider>(
      builder: (context, provider, _) {
        final toast = provider.toast;
        if (toast == null) return const SizedBox.shrink();

        Color iconColor;
        IconData icon;
        switch (toast.type) {
          case 'success':
            iconColor = AppColors.darkSuccess;
            icon = Icons.check_circle_outline;
            break;
          case 'error':
            iconColor = AppColors.darkDanger;
            icon = Icons.warning_amber_outlined;
            break;
          default:
            iconColor = AppColors.darkInfo;
            icon = Icons.info_outline;
        }

        return Positioned(
          bottom: 48,
          left: 0,
          right: 0,
          child: Material(
            color: Colors.transparent,
            child: Center(
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                decoration: BoxDecoration(
                  color: Theme.of(context).brightness == Brightness.dark
                      ? const Color(0xFF2A2A3A)
                      : const Color(0xFFFFFFFF),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: AppTheme.borderPrimary(context),
                  ),
                  boxShadow: [
                    BoxShadow(
                      color: Colors.black.withOpacity(0.3),
                      blurRadius: 8,
                      offset: const Offset(0, 2),
                    ),
                  ],
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(icon, size: 16, color: iconColor),
                    const SizedBox(width: 8),
                    Text(
                      toast.message,
                      style: TextStyle(
                        color: AppTheme.textPrimary(context),
                        fontSize: 12,
                      ),
                    ),
                    const SizedBox(width: 8),
                    GestureDetector(
                      onTap: provider.dismissToast,
                      child: Icon(
                        Icons.close,
                        size: 14,
                        color: AppTheme.textSecondary(context),
                      ),
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
