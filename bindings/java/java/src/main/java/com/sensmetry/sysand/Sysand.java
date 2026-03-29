// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.sensmetry.sysand;

public class Sysand {

    static {
        NativeLoader.load("sysand");
    }

    /**
     * Initialize a new project in the specified directory.
     *
     * @param name      The name of the project.
     * @param publisher The publisher. If {@code null}, default value will be used.
     * @param version   The version in SemVer 2.0.0 format.
     * @param license   SPDX license identifier. May be {@code null}.
     * @param path      Directory path to initialize the project in.
     */
    public static native void init(String name, String publisher, String version, String license, String path)
            throws com.sensmetry.sysand.exceptions.SysandException;

    public static void init(String name, String publisher, String version, String license, java.nio.file.Path path)
            throws com.sensmetry.sysand.exceptions.SysandException {
        init(name, publisher, version, license, path.toString());
    }

    /**
     * Get the default environment directory name ({@code sysand_env}).
     */
    public static native String defaultEnvName();

    /**
     * Create a local sysand_env environment directory.
     */
    public static native void env(String path) throws com.sensmetry.sysand.exceptions.SysandException;

    public static void env(java.nio.file.Path path)
            throws com.sensmetry.sysand.exceptions.SysandException {
        env(path.toString());
    }

    /**
     * Get absolute paths of all projects in a workspace.
     */
    private static native String[] workspaceProjectPaths(String workspacePath)
            throws com.sensmetry.sysand.exceptions.SysandException;

    public static String[] workspaceProjectPaths(java.nio.file.Path workspacePath)
            throws com.sensmetry.sysand.exceptions.SysandException {
        return workspaceProjectPaths(workspacePath.toString());
    }

    /**
     * Set the index field in a project's .meta.json file.
     */
    private static native void setProjectIndex(String projectPath, java.util.LinkedHashMap<String, String> index)
            throws com.sensmetry.sysand.exceptions.SysandException;

    public static void setProjectIndex(java.nio.file.Path projectPath,
            java.util.LinkedHashMap<String, String> index)
            throws com.sensmetry.sysand.exceptions.SysandException {
        setProjectIndex(projectPath.toString(), index);
    }

    /**
     * Build a KPAR archive from a project.
     */
    private static native void buildProject(String outputPath, String projectPath, String compression)
            throws com.sensmetry.sysand.exceptions.SysandException;

    public static void buildProject(java.nio.file.Path outputPath, java.nio.file.Path projectPath,
            com.sensmetry.sysand.model.CompressionMethod compression)
            throws com.sensmetry.sysand.exceptions.SysandException {
        buildProject(outputPath.toString(), projectPath.toString(), compression.toString());
    }

    /**
     * Build KPAR archives for all projects in a workspace.
     */
    private static native void buildWorkspace(String outputPath, String workspacePath, String compression)
            throws com.sensmetry.sysand.exceptions.SysandException;

    public static void buildWorkspace(java.nio.file.Path outputPath, java.nio.file.Path workspacePath,
            com.sensmetry.sysand.model.CompressionMethod compression)
            throws com.sensmetry.sysand.exceptions.SysandException {
        buildWorkspace(outputPath.toString(), workspacePath.toString(), compression.toString());
    }
}
