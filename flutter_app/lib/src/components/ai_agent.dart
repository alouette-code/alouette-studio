import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class AiAgentWidget extends StatelessWidget {
  const AiAgentWidget({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Theme.of(context).brightness == Brightness.dark
          ? const Color(0xFF14141E)
          : const Color(0xFFFAFAFC),
      child: Column(
        children: [
          // Header with back button
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
                Icon(Icons.arrow_back, size: 14, color: AppTheme.textSecondary(context)),
                const SizedBox(width: 8),
                Text('AI Agent', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 11, fontWeight: FontWeight.w600)),
                const Spacer(),
                Icon(Icons.history, size: 14, color: AppTheme.textSecondary(context)),
              ],
            ),
          ),
          // Chat area
          Expanded(
            child: Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.auto_awesome, size: 48, color: AppColors.darkAccentPurple.withOpacity(0.3)),
                  const SizedBox(height: 12),
                  Text('AI Agent Assistant', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 14, fontWeight: FontWeight.w600)),
                  const SizedBox(height: 4),
                  Text('Agent session & LLM integration coming via FRB', style: TextStyle(color: AppTheme.textSecondary(context), fontSize: 11)),
                ],
              ),
            ),
          ),
          // Input area
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              border: Border(
                top: BorderSide(color: AppTheme.borderPrimary(context)),
              ),
            ),
            child: Row(
              children: [
                Expanded(
                  child: Container(
                    height: 32,
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    decoration: BoxDecoration(
                      color: AppTheme.hoverColor(context),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: TextField(
                      style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 12),
                      decoration: InputDecoration(
                        hintText: 'Message AI Agent...',
                        hintStyle: TextStyle(color: AppTheme.textSecondary(context), fontSize: 12),
                        border: InputBorder.none,
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(vertical: 6),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: AppColors.darkAccent,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: const Icon(Icons.arrow_upward, size: 14, color: Colors.white),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class AiAgentTabWidget extends StatelessWidget {
  final String filePath;
  const AiAgentTabWidget({super.key, required this.filePath});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Text('Agent Session: $filePath', style: TextStyle(color: AppTheme.textPrimary(context), fontSize: 12)),
      ),
    );
  }
}
