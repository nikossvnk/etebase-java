package com.etebase.client;

import com.etebase.client.exceptions.Base64Exception;
import org.junit.Test;

import java.nio.charset.StandardCharsets;

import static org.junit.Assert.*;

public class NativeBindingsTest {
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
