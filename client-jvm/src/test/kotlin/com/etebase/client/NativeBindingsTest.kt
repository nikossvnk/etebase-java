package com.etebase.client

import java.io.File
import java.io.IOException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/** A canned in-process server: answers every request with [respond] and records it. */
private class FakeHttpClient(
    private val respond: (Request) -> HttpResponse = { HttpResponse(200u, ByteArray(0)) },
) : HttpClient {
    data class Request(val method: String, val url: String, val authToken: String?, val body: ByteArray?)

    private val recorded = mutableListOf<Request>()
    val requests: List<Request> get() = synchronized(recorded) { recorded.toList() }

    private fun handle(request: Request): HttpResponse {
        synchronized(recorded) { recorded += request }
        return respond(request)
    }

    override fun get(url: String, authToken: String?) = handle(Request("GET", url, authToken, null))
    override fun post(url: String, authToken: String?, body: ByteArray) =
        handle(Request("POST", url, authToken, body))
    override fun put(url: String, authToken: String?, body: ByteArray) =
        handle(Request("PUT", url, authToken, body))
    override fun patch(url: String, authToken: String?, body: ByteArray) =
        handle(Request("PATCH", url, authToken, body))
    override fun delete(url: String, authToken: String?) = handle(Request("DELETE", url, authToken, null))
}

private const val SERVER = "http://etebase.invalid/"

// The stored session of etebase-rs's `test_user` fixture (tests/common/mod.rs): restoring it needs
// no network, so collections and items can be created offline against a fake HttpClient.
private const val STORED_SESSION =
    "gqd2ZXJzaW9uAa1lbmNyeXB0ZWREYXRhxQGmmvZjsltGTbmbHECPtUBlTgICvtJHKHp246hnKKiDeJL90CrIOPzjRW6m" +
        "QMDX0SnQ8S32YEVLS2Ji5jzVfZOkyWzePAeSZmDpUxZd8N5WJ0BiuMKauG9UvXzdAGgVAVH9YA3dzPbAZUtpoU2W94eq" +
        "bupPCEUsjraLnLFW9g2UVrh35z4OW9QoC_0vgzqigpWySkTdJ_FjmQqalbuQF9CaTFJcngMnBy7uos4tKw53RawDQ_Ed" +
        "wuRQLJrVGP-9zQKzyi-Y5X_8eWImGcjHYZvLbN6O0uDxEDfcg0dQGaBB7YV94akSKIjPRHebvXYoPSjI-r0YkA9Q_-ti" +
        "aGxSwIFq-uVgWzOX9tq4dXsVP-2QffhV8Bx1hUHTHOyd6TCfEqQ3nWWaLsqA9yAoDg-XAPXHVffFwJ5b3accJU_Y5H8_" +
        "w6PmbBdrVFyN7lP6JB6yGBDs3gpT4osNwet1rRR_ueERr1ThSZcqdCwjfhOTZq0p5R3SAnzAnUJo0LUUBkuzNGeMCSlI" +
        "PJgE4HjOWXgLUYxZOlbDIN6yHp4SGIiAepSGOUrcjmRCQaA"
private const val SESSION_KEY = "YA0cMVJ9Q_SKYJCvKNni9y13vf62rEIq8M6kjtT14b4"

// msgpack {"data": [], "done": true}: an empty ItemListResponse.
private val EMPTY_ITEM_LIST = byteArrayOf(
    0x82.toByte(), 0xa4.toByte(), 'd'.code.toByte(), 'a'.code.toByte(), 't'.code.toByte(), 'a'.code.toByte(),
    0x90.toByte(), 0xa4.toByte(), 'd'.code.toByte(), 'o'.code.toByte(), 'n'.code.toByte(), 'e'.code.toByte(),
    0xc3.toByte(),
)

private fun ByteArray.contains(text: String): Boolean = String(this, Charsets.ISO_8859_1).contains(text)

class NativeBindingsTest {
    private val unicode = "Calendar 🗓 café\u0000end"

    private fun offlineItemManager(http: FakeHttpClient): ItemManager {
        val account = Account.restore(Client(SERVER, http), STORED_SESSION, fromBase64(SESSION_KEY))
        val collectionManager = account.collectionManager()
        val collection = collectionManager.create("etebase.test", ItemMetadata(name = "test"), ByteArray(0))
        return collectionManager.itemManager(collection)
    }

    @Test
    fun base64RoundTripsBytesAndRejectsInvalidInput() {
        val bytes = unicode.toByteArray()
        assertArrayEquals(bytes, fromBase64(toBase64(bytes)))
        val error = assertThrows(EtebaseException.Base64::class.java) { fromBase64("@@invalid@@") }
        assertTrue(error.message!!.isNotEmpty())
    }

    @Test
    fun utilitiesCallIntoNative() {
        assertEquals(32, randomBytes(32u).size)
        assertEquals("https://api.etebase.com/", defaultServerUrl())
        assertTrue(prettyFingerprint(randomBytes(32u)).isNotBlank())
    }

    @Test
    fun invalidServerUrlsMapToUrlParse() {
        val http = FakeHttpClient()
        val scheme = assertThrows(EtebaseException.UrlParse::class.java) { Client("ftp://example.com", http) }
        assertTrue(scheme.message!!, scheme.message!!.contains("http or https"))
        assertThrows(EtebaseException.UrlParse::class.java) { Client("not a url", http) }
        assertThrows(EtebaseException.UrlParse::class.java) { Client(SERVER, http).setServerUrl("::") }
    }

    @Test
    fun httpClientReceivesRequestsAndStatusesMapToResults() {
        var status: UShort = 200u
        val http = FakeHttpClient { HttpResponse(status, ByteArray(0)) }
        val client = Client(SERVER, http)
        assertTrue(isEtebaseServer(client))
        status = 404u
        assertFalse(isEtebaseServer(client))
        status = 503u
        assertThrows(EtebaseException.TemporaryServerException::class.java) { isEtebaseServer(client) }
        assertEquals(List(3) { "GET ${SERVER}api/v1/authentication/is_etebase/" }, http.requests.map { "${it.method} ${it.url}" })
        assertNull(http.requests.first().authToken)
    }

    @Test
    fun throwingHttpClientMapsToConnection() {
        val io = Client(SERVER, FakeHttpClient { throw IOException("network unreachable") })
        val ioError = assertThrows(EtebaseException.Connection::class.java) { isEtebaseServer(io) }
        assertEquals("java.io.IOException: network unreachable", ioError.message)

        val declared = Client(SERVER, FakeHttpClient { throw HttpClientException.Connection("TLS failure") })
        val declaredError = assertThrows(EtebaseException.Connection::class.java) { isEtebaseServer(declared) }
        assertEquals("TLS failure", declaredError.message)
    }

    @Test
    fun itemMetadataRoundTripsThroughNative() {
        val itemManager = offlineItemManager(FakeHttpClient())
        val full = ItemMetadata(unicode, unicode, 1_900_000_000_123L, "", "#ff0000")
        val item = itemManager.create(full, unicode.toByteArray())
        assertEquals(full, item.meta())
        assertEquals(unicode, String(item.content()))

        val empty = ItemMetadata()
        item.setMeta(empty)
        assertEquals(empty, item.meta())
        assertNull(item.meta().mtime)
        item.setMeta(full.copy(mtime = Long.MIN_VALUE, name = null))
        assertEquals(full.copy(mtime = Long.MIN_VALUE, name = null), item.meta())
    }

    @Test
    fun fetchOptionsRoundTripThroughNative() {
        val http = FakeHttpClient { HttpResponse(404u, ByteArray(0)) }
        val itemManager = offlineItemManager(http)
        val options = FetchOptions(
            limit = ULong.MAX_VALUE,
            stoken = "st&oken",
            iterator = unicode,
            prefetch = PrefetchOption.MEDIUM,
            withCollection = true,
        )
        assertThrows(EtebaseException.NotFound::class.java) { itemManager.list(options) }
        assertThrows(EtebaseException.NotFound::class.java) { itemManager.list() }
        assertThrows(EtebaseException.NotFound::class.java) { itemManager.list(FetchOptions(prefetch = PrefetchOption.AUTO)) }
        val (withOptions, withoutOptions, autoPrefetch) = http.requests.map { java.net.URI(it.url) }
        assertEquals(
            "limit=18446744073709551615&prefetch=medium&withCollection=true&stoken=st%26oken&iterator=" +
                java.net.URLEncoder.encode(unicode, Charsets.UTF_8),
            withOptions.rawQuery,
        )
        assertNull(withoutOptions.rawQuery)
        assertEquals("prefetch=auto", autoPrefetch.rawQuery)
    }

    @Test(timeout = 30_000)
    fun repeatedItemsInBatchesNeitherDeadlockNorLoseSavedEtags() {
        val http = FakeHttpClient { request ->
            if (request.url.endsWith("fetch_updates/")) HttpResponse(200u, EMPTY_ITEM_LIST) else HttpResponse(200u, ByteArray(0))
        }
        val itemManager = offlineItemManager(http)
        val item = itemManager.create(ItemMetadata(name = "one"), "first".toByteArray())
        val other = itemManager.create(ItemMetadata(name = "two"), "other".toByteArray())

        // The same item twice in items and once more in deps: one lock, shared references.
        itemManager.batch(listOf(item, item, other), deps = listOf(item, other))
        val savedEtag = item.etag()
        item.setContent("second".toByteArray())
        assertNotEquals(savedEtag, item.etag())
        itemManager.transaction(listOf(item, item), deps = listOf(item))
        assertTrue(itemManager.fetchUpdates(listOf(item, item, other)).data.isEmpty())

        val (_, transaction, fetchUpdates) = http.requests
        // batch() marked the Kotlin-held item saved, so the transaction sends the saved etag,
        // which is no longer the uid of any revision it uploads.
        assertTrue(transaction.url.endsWith("/transaction/"))
        assertTrue(transaction.body!!.contains(savedEtag))
        // Likewise transaction() marked it saved again; fetch_updates sends only uids and etags.
        assertTrue(fetchUpdates.url.endsWith("/fetch_updates/"))
        assertTrue(fetchUpdates.body!!.contains(item.etag()))
    }

    @Test(timeout = 60_000)
    fun concurrentBatchesWithOppositeOrderDoNotDeadlock() {
        val itemManager = offlineItemManager(FakeHttpClient())
        val a = itemManager.create(ItemMetadata(name = "a"), ByteArray(0))
        val b = itemManager.create(ItemMetadata(name = "b"), ByteArray(0))
        val pool = Executors.newFixedThreadPool(2)
        val start = CountDownLatch(1)
        try {
            val jobs = listOf(listOf(a, b) to listOf(b), listOf(b, a) to listOf(a)).map { (items, deps) ->
                pool.submit {
                    start.await()
                    repeat(200) { itemManager.batch(items, deps) }
                }
            }
            start.countDown()
            jobs.forEach { it.get(50, TimeUnit.SECONDS) }
        } finally {
            pool.shutdownNow()
        }
    }

    @Test
    fun generatedBindingsUseCleanersNotFinalizers() {
        val sources = File(System.getProperty("etebase.bindingsDir")).walk().filter { it.extension == "kt" }.toList()
        assertTrue("no generated sources found", sources.isNotEmpty())
        val code = sources.joinToString("\n") { it.readText() }
        assertFalse(Regex("""\bfun\s+finalize\s*\(""").containsMatchIn(code))
        assertTrue(code.contains("java.lang.ref.Cleaner"))
    }
}
