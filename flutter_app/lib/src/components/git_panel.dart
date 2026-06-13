import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class GitPanelWidget extends StatelessWidget {
  const GitPanelWidget({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: Column(
        children: [
          // Header
          Container(
            height: 32,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(color: AppTheme.borderPrimary(context)),
              ),
            ),
            child: Row(
              children: [
                Icon(Icons.account_tree_outlined, size: 14, color: AppTheme.textSecondary(context)),
                const SizedBox(width: 8),
                Text('Git', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 11, fontWeight: FontWeight.w600)),
                const Spacer(),
                Icon(Icons.refresh, size: 14, color: AppTheme.textSecondary(context)),
              ],
            ),
          ),
          // Content
          Expanded(
            child: Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.account_tree_outlined, size: 48, color: AppTheme.textSecondary(context).withOpacity(0.3)),
                  const SizedBox(height: 12),
                  Text('Git Management', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 14, fontWeight: FontWeight.w600)),
                  const SizedBox(height: 4),
                  Text('Stage, commit, push/pull via FRB', style: TextStyle(color: AppTheme.textSecondary(context), fontSize: 11)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
