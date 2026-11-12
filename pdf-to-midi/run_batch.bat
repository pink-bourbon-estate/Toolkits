@echo off
REM Usage: run_batch.bat "C:\path\to\folder"
python -I "%~dp0batch_tree.py" %1 --jobs 4 --timeout 600
pause
