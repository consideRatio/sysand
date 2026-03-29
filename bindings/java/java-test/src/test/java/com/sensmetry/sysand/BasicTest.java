// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package com.sensmetry.sysand;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeAll;

import com.sensmetry.sysand.model.CompressionMethod;

import static org.junit.jupiter.api.Assertions.*;

import java.util.regex.Pattern;
import java.nio.file.Files;

public class BasicTest {

    static SysandClient client;

    @BeforeAll
    static void setup() {
        client = new SysandClient();
    }

    @Test
    public void testBasicInit() {
        try {
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-init");
            client.init("test", "a", "1.0.0", null, tempDir);

            assertTrue(Files.exists(tempDir.resolve(".project.json")));
            assertTrue(Files.exists(tempDir.resolve(".meta.json")));

            String projectJson = new String(Files.readAllBytes(tempDir.resolve(".project.json")));
            assertEquals(
                    "{\n  \"name\": \"test\",\n  \"publisher\": \"a\",\n  \"version\": \"1.0.0\",\n  \"usage\": []\n}\n",
                    projectJson);

            String metaJson = new String(Files.readAllBytes(tempDir.resolve(".meta.json")));
            Pattern regex = Pattern.compile(
                    "\\{\\s*\"index\":\\s*\\{\\},\\s*\"created\":\\s*\"\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}.\\d{6,9}Z\"\\s*\\}\n",
                    Pattern.DOTALL);
            assertTrue(regex.matcher(metaJson).matches());
        } catch (Exception e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testBasicEnv() {
        try {
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-env");
            java.nio.file.Path envPath = tempDir.resolve(client.env().defaultName());
            client.env().create(envPath);

            assertTrue(Files.exists(envPath.resolve("entries.txt")));
            String entries = new String(Files.readAllBytes(envPath.resolve("entries.txt")));
            assertEquals("", entries);
        } catch (Exception e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testProjectBuild() {
        try {
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-build");
            client.init("test_build", "a", "1.2.3", "MIT", tempDir);

            java.nio.file.Path kparPath = tempDir.resolve("test_build.kpar");
            client.build(kparPath, tempDir, CompressionMethod.DEFLATED);
            assertTrue(Files.exists(kparPath));
            assertTrue(Files.size(kparPath) > 0);
        } catch (Exception e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testSetProjectIndex() {
        try {
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-update-index");
            client.init("test_index", "a", "1.0.0", null, tempDir);

            java.util.LinkedHashMap<String, String> index = new java.util.LinkedHashMap<>();
            index.put("Foo", "src/Foo.sysml");
            index.put("Bar", "src/Bar.sysml");
            index.put("Baz", "src/sub/Baz.kerml");

            // setProjectIndex stays on Sysand for now (low-level operation)
            Sysand.setProjectIndex(tempDir, index);

            String metaJson = new String(Files.readAllBytes(tempDir.resolve(".meta.json")));
            assertTrue(metaJson.contains("\"Foo\": \"src/Foo.sysml\""));
            assertTrue(metaJson.contains("\"Bar\": \"src/Bar.sysml\""));
            assertTrue(metaJson.contains("\"Baz\": \"src/sub/Baz.kerml\""));
        } catch (Exception e) {
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
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-workspace-paths");

            java.nio.file.Path projA = tempDir.resolve("projA");
            java.nio.file.Path projB = tempDir.resolve("projB");
            Files.createDirectories(projA);
            Files.createDirectories(projB);
            client.init("projA", "a", "1.0.0", null, projA);
            client.init("projB", "a", "1.0.0", null, projB);

            writeWorkspaceJson(tempDir, "projA", "projB");

            String[] paths = client.workspace().projectPaths(tempDir);
            assertEquals(2, paths.length);

            java.util.Arrays.sort(paths);
            assertTrue(paths[0].endsWith("projA"));
            assertTrue(paths[1].endsWith("projB"));
            assertTrue(java.nio.file.Paths.get(paths[0]).isAbsolute());
            assertTrue(java.nio.file.Paths.get(paths[1]).isAbsolute());
        } catch (Exception e) {
            fail("Failed: " + e.getMessage());
        }
    }

    @Test
    public void testWorkspaceBuildAndIndex() {
        try {
            java.nio.file.Path tempDir = Files.createTempDirectory("sysand-test-workspace-index");

            java.nio.file.Path projA = tempDir.resolve("projA");
            java.nio.file.Path projB = tempDir.resolve("projB");
            Files.createDirectories(projA);
            Files.createDirectories(projB);
            client.init("projA", "a", "1.0.0", null, projA);
            client.init("projB", "a", "1.0.0", null, projB);

            writeWorkspaceJson(tempDir, "projA", "projB");

            // Set indexes via low-level API
            java.util.LinkedHashMap<String, String> indexA = new java.util.LinkedHashMap<>();
            indexA.put("Alpha", "src/Alpha.sysml");
            Sysand.setProjectIndex(projA, indexA);

            java.util.LinkedHashMap<String, String> indexB = new java.util.LinkedHashMap<>();
            indexB.put("Gamma", "lib/Gamma.kerml");
            Sysand.setProjectIndex(projB, indexB);

            // Verify via raw JSON
            String metaA = new String(Files.readAllBytes(projA.resolve(".meta.json")));
            assertTrue(metaA.contains("\"Alpha\""));
            assertFalse(metaA.contains("\"Gamma\""));

            String metaB = new String(Files.readAllBytes(projB.resolve(".meta.json")));
            assertTrue(metaB.contains("\"Gamma\""));
            assertFalse(metaB.contains("\"Alpha\""));
        } catch (Exception e) {
            fail("Failed: " + e.getMessage());
        }
    }
}
