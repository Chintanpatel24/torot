from setuptools import setup, find_packages

setup(
    name="torot",
    version="1.0.0",
    packages=find_packages(),
    entry_points={
        "console_scripts": [
            "torot=torot.cli:main",
        ],
    },
    install_requires=[
        "rich>=13.0.0",
        "textual>=0.40.0",
    ],
    python_requires=">=3.9",
)
