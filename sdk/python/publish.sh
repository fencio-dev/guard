#!/bin/bash
set -e

echo "🧹 Cleaning previous builds..."
rm -rf dist/ build/ *.egg-info

echo "📦 Building package..."
python -m build

echo "✅ Checking package..."
twine check dist/*

echo ""
echo "🚀 Ready to upload to PyPI!"
echo ""
echo "To upload to TestPyPI (recommended first time):"
echo "  twine upload --repository testpypi dist/*"
echo ""
echo "To upload to PyPI:"
echo "  twine upload dist/*"
echo ""
echo "You will be prompted for:"
echo "  Username: __token__"
echo "  Password: <your-pypi-api-token>"
