# MDMA

# How to Add and Load an Unpacked Browser Extension

This tutorial will guide you through the process of adding and loading an unpacked browser extension in Chrome and Firefox. This is useful for testing and developing extensions before packaging them for distribution.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Loading Unpacked Extension in Chrome](#loading-unpacked-extension-in-chrome)
3. [Loading Unpacked Extension in Firefox](#loading-unpacked-extension-in-firefox)
4. [Common Issues and Troubleshooting](#common-issues-and-troubleshooting)

## Prerequisites

Before you start, ensure you have the following:

- The extension files are ready and organized in a single folder.
- You have Google Chrome (or any other chromium based browser) and/or Mozilla Firefox installed on your computer.

## Loading Unpacked Extension in Chrome

1. **Open Chrome Browser**:
   Open your Chrome browser.

2. **Navigate to Extensions Page**:
   In the address bar, type `chrome://extensions/` and press Enter.

3. **Enable Developer Mode**:
   In the top right corner, toggle the switch to enable Developer Mode.

4. **Click on "Load unpacked"**:
   A new set of buttons will appear. Click on the `Load unpacked` button.

5. **Select Your Extension Folder**:
   A file dialog will appear. Navigate to the folder containing your extension files, select it, and click `Select Folder`.

6. **Verify Extension Loaded**:
   Your extension should now appear in the list with a description, version number, and options to disable, update, or remove it.

## Loading Unpacked Extension in Firefox

1. **Open Firefox Browser**:
   Open your Firefox browser.

2. **Navigate to About:Debugging Page**:
   In the address bar, type `about:debugging` and press Enter.

3. **Click on "This Firefox"**:
   In the sidebar, click on `This Firefox` (or `This Nightly` if you are using Firefox Nightly).

4. **Click on "Load Temporary Add-on"**:
   Click the `Load Temporary Add-on…` button.

5. **Select Your Extension File**:
   A file dialog will appear. Navigate to the folder containing your extension files, and select the manifest file (usually `manifest.json`).

6. **Verify Extension Loaded**:
   Your extension should now appear in the list with options to reload or remove it.

## Common Issues and Troubleshooting

- **Error Messages**:
    - **Manifest File Errors**: Ensure your `manifest.json` file is correctly formatted and contains all required fields.
    - **File Paths**: Verify that all paths in your `manifest.json` (like icons or background scripts) are correct and point to existing files.

- **Changes Not Reflected**:
    - **Chrome**: Click the `Reload` button on the extension in the `chrome://extensions/` page.
    - **Firefox**: Click the `Reload` button next to the extension in the `about:debugging` page.

- **Permission Issues**:
    - Make sure you have the necessary permissions specified in your `manifest.json`.
    - If you add new permissions, you may need to reload the extension for changes to take effect.

By following these steps, you should be able to successfully load and test your unpacked browser extension in both Chrome and Firefox. If you encounter any issues, refer to the browser's developer documentation for further guidance.