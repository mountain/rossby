#!/bin/bash
#
# Script to install Git hooks for the Rossby project
#

echo "Installing Git hooks for Rossby..."

# Create .git/hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy the pre-commit hook
cp hooks/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit

echo "Git hooks installed successfully!"
echo "Pre-commit hook will run tests, check formatting, and lint code before each commit."
echo "To skip hooks (not recommended), use: git commit --no-verify"
