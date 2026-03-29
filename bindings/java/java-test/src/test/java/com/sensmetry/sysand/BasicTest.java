// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.sensmetry.sysand;

import org.junit.jupiter.api.Test;

import com.sensmetry.sysand.model.CompressionMethod;

import static org.junit.jupiter.api.Assertions.*;

import java.util.regex.Pattern;
import java.nio.file.Files;

public class BasicTest {

    @Test
    public void testBasicInit() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-init");
            com.sensmetry.sysand.Sysand.init("test", "a", "1.0.0", null, tempDir);

            assertTrue(Files.exists(tempDir.resolve(".project.json")), "Project file should exist");
            assertTrue(Files.exists(tempDir.resolve(".meta.json")), "Metadata file should exist");

            String projectJson = new String(Files.readAllBytes(tempDir.resolve(".project.json")));
            assertEquals(
                    "{\n  \"name\": \"test\",\n  \"publisher\": \"a\",\n  \"version\": \"1.0.0\",\n  \"usage\": []\n}\n",
                    projectJson);

            String metaJson = new String(Files.readAllBytes(tempDir.resolve(".meta.json")));
            Pattern regex = Pattern.compile(
                    "\\{\\s*\"index\":\\s*\\{\\},\\s*\"created\":\\s*\"\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}.\\d{6,9}Z\"\\s*\\}\n",
                    Pattern.DOTALL);
            assertTrue(regex.matcher(metaJson).matches(), "Metadata file content should match expected pattern");
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testBasicEnv() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-env");
            java.nio.file.Path envPath = tempDir.resolve(com.sensmetry.sysand.Sysand.defaultEnvName());
            com.sensmetry.sysand.Sysand.env(envPath);

            assertTrue(Files.exists(envPath.resolve("entries.txt")), "Entries file should exist");
            String entries = new String(Files.readAllBytes(envPath.resolve("entries.txt")));
            assertEquals("", entries);
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testProjectBuild() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-build");
            com.sensmetry.sysand.Sysand.init("test_build", "a", "1.2.3", "MIT", tempDir);

            java.nio.file.Path kparPath = tempDir.resolve("test_build.kpar");
            com.sensmetry.sysand.Sysand.buildProject(kparPath, tempDir, CompressionMethod.DEFLATED);
            assertTrue(Files.exists(kparPath), "KPAR file should exist");
            assertTrue(Files.size(kparPath) > 0, "KPAR file should not be empty");
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testSetProjectIndex() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-update-index");
            com.sensmetry.sysand.Sysand.init("test_index", "a", "1.0.0", null, tempDir);

            java.util.LinkedHashMap<String, String> index = new java.util.LinkedHashMap<>();
            index.put("Foo", "src/Foo.sysml");
            index.put("Bar", "src/Bar.sysml");
            index.put("Baz", "src/sub/Baz.kerml");

            com.sensmetry.sysand.Sysand.setProjectIndex(tempDir, index);

            // Verify via raw JSON
            String metaJson = new String(Files.readAllBytes(tempDir.resolve(".meta.json")));
            assertTrue(metaJson.contains("\"Foo\": \"src/Foo.sysml\""), "meta.json should contain Foo");
            assertTrue(metaJson.contains("\"Bar\": \"src/Bar.sysml\""), "meta.json should contain Bar");
            assertTrue(metaJson.contains("\"Baz\": \"src/sub/Baz.kerml\""), "meta.json should contain Baz");
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }

    private void writeWorkspaceJson(java.nio.file.Path workspaceDir, String... projectNames)
            throws java.io.IOException {
        StringBuilder sb = new StringBuilder();
        sb.append("{\n  \"projects\": [\n");
        for (int i = 0; i < projectNames.length; i++) {
            sb.append("    {\"path\": \"").append(projectNames[i])
                    .append("\", \"iris\": [\"urn:test:").append(projectNames[i]).append("\"]}");
            if (i < projectNames.length - 1)
                sb.append(",");
            sb.append("\n");
        }
        sb.append("  ]\n}\n");
        Files.write(workspaceDir.resolve(".workspace.json"), sb.toString().getBytes());
    }

    @Test
    public void testWorkspaceProjectPaths() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-workspace-paths");

            java.nio.file.Path projA = tempDir.resolve("projA");
            java.nio.file.Path projB = tempDir.resolve("projB");
            Files.createDirectories(projA);
            Files.createDirectories(projB);
            com.sensmetry.sysand.Sysand.init("projA", "a", "1.0.0", null, projA);
            com.sensmetry.sysand.Sysand.init("projB", "a", "1.0.0", null, projB);

            writeWorkspaceJson(tempDir, "projA", "projB");

            String[] paths = com.sensmetry.sysand.Sysand.workspaceProjectPaths(tempDir);
            assertEquals(2, paths.length);

            java.util.Arrays.sort(paths);
            assertTrue(paths[0].endsWith("projA"), "First path should end with projA: " + paths[0]);
            assertTrue(paths[1].endsWith("projB"), "Second path should end with projB: " + paths[1]);
            assertTrue(java.nio.file.Paths.get(paths[0]).isAbsolute(), "Paths should be absolute");
            assertTrue(java.nio.file.Paths.get(paths[1]).isAbsolute(), "Paths should be absolute");
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testSetWorkspaceProjectIndexes() {
        try {
            java.nio.file.Path tempDir = java.nio.file.Files.createTempDirectory("sysand-test-workspace-index");

            java.nio.file.Path projA = tempDir.resolve("projA");
            java.nio.file.Path projB = tempDir.resolve("projB");
            Files.createDirectories(projA);
            Files.createDirectories(projB);
            com.sensmetry.sysand.Sysand.init("projA", "a", "1.0.0", null, projA);
            com.sensmetry.sysand.Sysand.init("projB", "a", "1.0.0", null, projB);

            writeWorkspaceJson(tempDir, "projA", "projB");

            String[] paths = com.sensmetry.sysand.Sysand.workspaceProjectPaths(tempDir);
            assertEquals(2, paths.length);

            java.util.LinkedHashMap<String, String> indexA = new java.util.LinkedHashMap<>();
            indexA.put("Alpha", "src/Alpha.sysml");
            indexA.put("Beta", "src/Beta.sysml");

            java.util.LinkedHashMap<String, String> indexB = new java.util.LinkedHashMap<>();
            indexB.put("Gamma", "lib/Gamma.kerml");

            java.util.Arrays.sort(paths);
            com.sensmetry.sysand.Sysand.setProjectIndex(java.nio.file.Paths.get(paths[0]), indexA);
            com.sensmetry.sysand.Sysand.setProjectIndex(java.nio.file.Paths.get(paths[1]), indexB);

            // Verify via raw JSON
            String metaA = new String(
                    Files.readAllBytes(java.nio.file.Paths.get(paths[0]).resolve(".meta.json")));
            assertTrue(metaA.contains("\"Alpha\""), "projA should contain Alpha");
            assertTrue(metaA.contains("\"Beta\""), "projA should contain Beta");
            assertFalse(metaA.contains("\"Gamma\""), "projA should not contain Gamma");

            String metaB = new String(
                    Files.readAllBytes(java.nio.file.Paths.get(paths[1]).resolve(".meta.json")));
            assertTrue(metaB.contains("\"Gamma\""), "projB should contain Gamma");
            assertFalse(metaB.contains("\"Alpha\""), "projB should not contain Alpha");
        } catch (java.io.IOException e) {
            fail("Failed: " + e.getMessage());
        } catch (com.sensmetry.sysand.exceptions.SysandException e) {
            fail("Failed: " + e.getMessage());
        }
    }
}
