#!/usr/bin/env python3

import os
import json
from pathlib import Path

def generate_manifest(content_dir, output_file):
    """Generate a manifest.json from the content directory structure"""

    content_path = Path(content_dir)
    files = []
    directories = set()

    # Walk through content directory
    for item in content_path.rglob('*'):
        if item.is_file():
            # Get relative path from content directory
            relative = item.relative_to(content_path)

            # Get directory path (empty string for root)
            if relative.parent == Path('.'):
                dir_path = ""
            else:
                dir_path = str(relative.parent)
                # Add all parent directories
                parts = relative.parts[:-1]
                for i in range(len(parts)):
                    directories.add('/'.join(parts[:i+1]))

            files.append({
                "name": item.name,
                "path": dir_path
            })

    # Create manifest structure
    manifest = {
        "files": sorted(files, key=lambda x: (x['path'], x['name'])),
        "directories": sorted(list(directories))
    }

    # Ensure output directory exists
    output_path = Path(output_file)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Write manifest
    with open(output_file, 'w') as f:
        json.dump(manifest, f, indent=2)

    print(f"✓ Generated manifest with {len(files)} files and {len(directories)} directories")
    print(f"✓ Manifest saved to {output_file}")

if __name__ == "__main__":
    generate_manifest("site/content", "dist/content/manifest.json")
