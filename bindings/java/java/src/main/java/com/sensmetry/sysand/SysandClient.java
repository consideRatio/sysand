// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.sensmetry.sysand;

import com.sensmetry.sysand.exceptions.SysandException;
import com.sensmetry.sysand.model.CompressionMethod;

/**
 * Entry point for the sysand Java API.
 *
 * <p>Root-level commands are methods on this class. Namespaced commands
 * are accessed via accessor methods: {@code client.source().add(...)},
 * {@code client.env().create(...)}, etc.</p>
 *
 * <pre>{@code
 * SysandClient client = new SysandClient();
 * client.init("sensors", null, "1.0.0", null, path);
 * client.build(outputPath, projectPath, CompressionMethod.DEFLATED);
 * client.source().add(...);  // future
 * client.env().create(envPath);
 * }</pre>
 */
public class SysandClient {

    private final Source source = new Source();
    private final Env env = new Env();
    private final Workspace workspace = new Workspace();

    public SysandClient() {
        // Ensure native library is loaded
        Sysand.defaultEnvName(); // triggers static initializer
    }

    // -- Root commands --

    /**
     * Initialize a new project.
     */
    public void init(String name, String publisher, String version, String license, String path)
            throws SysandException {
        Sysand.init(name, publisher, version, license, path);
    }

    public void init(String name, String publisher, String version, String license, java.nio.file.Path path)
            throws SysandException {
        Sysand.init(name, publisher, version, license, path);
    }

    /**
     * Build a KPAR archive from a project.
     */
    public void build(java.nio.file.Path outputPath, java.nio.file.Path projectPath, CompressionMethod compression)
            throws SysandException {
        Sysand.buildProject(outputPath, projectPath, compression);
    }

    // -- Namespace accessors --

    public Source source() {
        return source;
    }

    public Env env() {
        return env;
    }

    public Workspace workspace() {
        return workspace;
    }

    // -- Source namespace --

    public static class Source {
        // Future: add(paths, opts), remove(paths)
        // These will be wired when source add/remove JNI functions are added.
    }

    // -- Env namespace --

    public static class Env {

        /**
         * Get the default environment directory name.
         */
        public String defaultName() {
            return Sysand.defaultEnvName();
        }

        /**
         * Create a local sysand_env environment directory.
         */
        public void create(String path) throws SysandException {
            Sysand.env(path);
        }

        public void create(java.nio.file.Path path) throws SysandException {
            Sysand.env(path);
        }
    }

    // -- Workspace namespace --

    public static class Workspace {

        /**
         * Get absolute paths of all projects in a workspace.
         */
        public String[] projectPaths(java.nio.file.Path workspacePath) throws SysandException {
            return Sysand.workspaceProjectPaths(workspacePath);
        }

        /**
         * Build KPAR archives for all projects in a workspace.
         */
        public void build(java.nio.file.Path outputPath, java.nio.file.Path workspacePath,
                CompressionMethod compression) throws SysandException {
            Sysand.buildWorkspace(outputPath, workspacePath, compression);
        }
    }
}
