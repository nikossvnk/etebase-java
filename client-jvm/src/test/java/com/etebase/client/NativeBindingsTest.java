package com.etebase.client;

import com.etebase.client.exceptions.Base64Exception;
import org.junit.Test;
import java.io.File;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.regex.Pattern;

import java.nio.charset.StandardCharsets;

import static org.junit.Assert.*;

public class NativeBindingsTest {
    @Test
    public void nativeLoadingDoesNotRequireApi24OptionalClasses() throws Exception {
        URL[] classpath = Arrays.stream(System.getProperty("etebase.test.classpath")
                        .split(Pattern.quote(File.pathSeparator)))
                .map(path -> {
                    try {
                        return Path.of(path).toUri().toURL();
                    } catch (java.net.MalformedURLException e) {
                        throw new IllegalArgumentException(e);
                    }
                }).toArray(URL[]::new);
        // JNI_OnLoad uses the loading class's loader. Emulate the API 23 class set,
        // not an application-level mock of the native call.
        try (URLClassLoader loader = new URLClassLoader(classpath, ClassLoader.getPlatformClassLoader()) {
            @Override
            protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
                if (name.startsWith("java.util.Optional"))
                    throw new ClassNotFoundException("Not available on API 23: " + name);
                return super.loadClass(name, resolve);
            }
        }) {
            Class<?> utils = Class.forName("com.etebase.client.Utils", true, loader);
            Object encoded = utils.getMethod("toBase64", byte[].class)
                    .invoke(null, (Object) new byte[]{1, 2, 3});
            assertEquals("AQID", encoded);
        }
    }

    @Test
    public void base64LoadsNativeLibraryWithoutCreatingClient() {
        byte[] bytes = "Calendar 🗓 café\u0000end".getBytes(StandardCharsets.UTF_8);
        assertArrayEquals(bytes, Utils.fromBase64(Utils.toBase64(bytes)));
        assertThrows(Base64Exception.class, () -> Utils.fromBase64("@@invalid@@"));
    }

    @Test
    public void metadataPreservesUnicodeAndNullStrings() {
        ItemMetadata metadata = new ItemMetadata();
        assertNull(metadata.getName());
        String name = "Calendar 🗓 café\u0000end";
        metadata.setName(name);
        assertEquals(name, metadata.getName());
        metadata.setName(null);
        assertNull(metadata.getName());
    }

    @Test
    public void metadataPreservesNullable64BitTimestamps() {
        ItemMetadata metadata = new ItemMetadata();
        assertNull(metadata.getMtime());
        Long timestamp = 1_900_000_000_123L;
        metadata.setMtime(timestamp);
        assertEquals(timestamp, metadata.getMtime());
        metadata.setMtime(null);
        assertNull(metadata.getMtime());
    }
}
