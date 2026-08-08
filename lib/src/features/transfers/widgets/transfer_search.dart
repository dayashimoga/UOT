// Transfer Search & Filter Bar Widget
import 'package:flutter/material.dart';

class TransferSearchBar extends StatelessWidget {
  const TransferSearchBar({
    super.key,
    required this.onQueryChanged,
    required this.onFilterSelected,
    this.selectedFilter,
  });

  final ValueChanged<String> onQueryChanged;
  final ValueChanged<String?> onFilterSelected;
  final String? selectedFilter;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        children: [
          Expanded(
            child: TextField(
              onChanged: onQueryChanged,
              decoration: InputDecoration(
                hintText: 'Search transfers or devices...',
                prefixIcon: const Icon(Icons.search_rounded, size: 20),
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 10,
                ),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                  borderSide: BorderSide.none,
                ),
                filled: true,
                fillColor: colorScheme.surfaceContainerHighest,
              ),
            ),
          ),
          const SizedBox(width: 8),
          PopupMenuButton<String>(
            icon: Icon(Icons.filter_list_rounded, color: colorScheme.primary),
            tooltip: 'Filter by Status',
            onSelected: onFilterSelected,
            itemBuilder: (context) => [
              const PopupMenuItem(value: null, child: Text('All Transfers')),
              const PopupMenuItem(value: 'InProgress', child: Text('Active')),
              const PopupMenuItem(value: 'Completed', child: Text('Completed')),
              const PopupMenuItem(value: 'Failed', child: Text('Failed')),
              const PopupMenuItem(value: 'Paused', child: Text('Paused')),
            ],
          ),
        ],
      ),
    );
  }
}
