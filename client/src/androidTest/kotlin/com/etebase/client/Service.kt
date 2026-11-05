package com.etebase.client

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.util.UUID
import okhttp3.OkHttpClient
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class Service {
    private fun newClient(): Client = createClient(httpClient, SERVER_URL)

    private fun newAccount(client: Client): Account {
        val username = "android-" + UUID.randomUUID()
        return Account.signup(client, User(username, "$username@example.com"), "test password only")
    }

    @Test
    fun testSmoketest() {
        val etebase = newAccount(newClient())
        etebase.forceServerUrl(SERVER_URL)
        etebase.fetchToken()
        val colMgr = etebase.collectionManager()
        val col = colMgr.create(COL_TYPE, ItemMetadata(), "Something".toByteArray())
        assertEquals("Something", String(col.content()))
        colMgr.upload(col, FetchOptions(prefetch = PrefetchOption.AUTO))
        var colList = colMgr.list(COL_TYPE)
        assertNotEquals(0, colList.data.size)
        colList = colMgr.list(COL_TYPE, FetchOptions(stoken = colList.stoken))
        assertEquals(0, colList.data.size)

        val col2 = colMgr.fetch(col.uid())
        assertEquals("Something", String(col2.content()))
        col2.setContent("Something else".toByteArray())
        colMgr.transaction(col2)

        val itMgr = colMgr.itemManager(col)
        val item = itMgr.create(ItemMetadata(itemType = "Bla"), "Something item".toByteArray())
        assertNotEquals("", item.uid())
        assertEquals("Something item", String(item.content()))
        itMgr.batch(listOf(item))
        item.setContent("Something item2".toByteArray())
        itMgr.transaction(listOf(item), deps = emptyList())
        var itemList = itMgr.list()
        assertEquals(1, itemList.data.size)
        val it2First = itemList.data[0]
        itemList = itMgr.list(FetchOptions(stoken = itemList.stoken))
        assertEquals(0, itemList.data.size)
        assertEquals("Something item2", String(it2First.content()))

        etebase.logout()
    }

    @Test
    fun testRemovedCollections() {
        val etebase = newAccount(newClient())
        etebase.forceServerUrl(SERVER_URL)
        etebase.fetchToken()
        val colMgr = etebase.collectionManager()
        val col = colMgr.create(COL_TYPE, ItemMetadata(), "Something".toByteArray())
        colMgr.upload(col)
        var colList = colMgr.list(COL_TYPE)
        colList = colMgr.list(COL_TYPE, FetchOptions(stoken = colList.stoken))
        assertEquals(0, colList.removedMemberships.size)

        // FIXME: actually test we get the removed memberships ever.

        etebase.logout()
    }

    @Test
    fun testMetaMtimeNull() {
        assertNull(ItemMetadata().mtime)
        assertNull(ItemMetadata(mtime = null).mtime)
        assertEquals(3243L, ItemMetadata(mtime = 3243L).mtime)
    }

    @Test
    fun testCache() {
        val client = newClient()
        var etebase = newAccount(client)
        etebase.forceServerUrl(SERVER_URL)
        etebase.fetchToken()
        var colMgr = etebase.collectionManager()
        var col = colMgr.create(COL_TYPE, ItemMetadata(), "Something".toByteArray())
        colMgr.upload(col)

        val length = colMgr.list(COL_TYPE).data.size
        assertTrue(length > 0)
        assertEquals(length, colMgr.listMulti(listOf("bad1", COL_TYPE, "bad2")).data.size)
        assertEquals(0, colMgr.listMulti(listOf("bad1", "bad2")).data.size)

        val cached = etebase.save()
        etebase = Account.restore(client, cached)
        colMgr = etebase.collectionManager()
        col = colMgr.fetch(col.uid())
        assertArrayEquals("Something".toByteArray(), col.content())
        col = colMgr.cacheLoad(colMgr.cacheSave(col))
        assertEquals(COL_TYPE, col.collectionType())

        val itMgr = colMgr.itemManager(col)
        var item = itMgr.create(ItemMetadata(itemType = "Bla"), "Something item".toByteArray())
        itMgr.batch(listOf(item))
        item = itMgr.cacheLoad(itMgr.cacheSaveWithContent(item))
        assertArrayEquals("Something item".toByteArray(), item.content())

        etebase.logout()
    }

    @Test
    fun testBase64() {
        assertArrayEquals("Test".toByteArray(), fromBase64(toBase64("Test".toByteArray())))
    }

    @Test
    fun base64Exception() {
        assertThrows(EtebaseException.Base64::class.java) { fromBase64("#@$@#$*@#$") }
    }

    @Test
    fun testUnauthorizedException() {
        val etebase = newAccount(newClient())
        etebase.forceServerUrl(SERVER_URL)
        etebase.fetchToken()
        etebase.logout()
        val collectionManager = etebase.collectionManager()
        assertThrows(EtebaseException.Unauthorized::class.java) { collectionManager.list(COL_TYPE) }
    }

    private companion object {
        val SERVER_URL: String = InstrumentationRegistry.getArguments()
            .getString("etebaseServerUrl", "http://10.0.2.2:3735")

        val httpClient: OkHttpClient = OkHttpClient.Builder()
            // don't allow redirects by default, because it would break PROPFIND handling
            .followRedirects(false)
            .build()

        const val COL_TYPE = "some.coltype"
    }
}
