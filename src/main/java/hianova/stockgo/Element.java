package hianova.stockgo;

import java.net.InetAddress;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.security.SecureRandom;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Optional;
import java.util.regex.Pattern;

import javax.crypto.Cipher;
import javax.crypto.spec.SecretKeySpec;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import io.ipfs.api.NamedStreamable.ByteArrayWrapper;

public class Element extends IPFSLayer {

    private ObjectNode ele;

    /*
     * String encrypt; {
     * String author;
     * String date;
     * String body;
     * String IP;
     * String agreement;
     * }
     */

    public Element() throws Exception {
        ele = new ObjectMapper().createObjectNode();

        ele.put("agreement", "CC BY-NC");
        ele.put("author", String.format("[%s](/ipfs/%s)",
                user.get("name"), user.get("ipns")));
    }

    public void post() throws Exception {
        var stockgo = (ArrayNode) profile.at("/stockgo");

        stockgo.add(ipfs.dag.put(ele.toPrettyString().getBytes()).hash.toString());
        syncIpns();
    }

    public Element encrypt(String passcodeIn) throws Exception {
        var json = new ObjectMapper().createObjectNode();

        if (!passcodeIn.isEmpty()) {
            var key = new byte[16];
            var secretKeySpec = new SecretKeySpec(key, "AES");
            var cipher = Cipher.getInstance("AES");

            new SecureRandom().nextBytes(key);
            cipher.init(Cipher.ENCRYPT_MODE, secretKeySpec);
            var part1 = ipfs.add(new ByteArrayWrapper(Arrays.copyOfRange(key, 0, 8)));
            var part2 = ipfs.add(new ByteArrayWrapper(Arrays.copyOfRange(key, 8, key.length)));
            json.put("encrypt",
                    part1.get(0).hash.toString() + part2.get(0).hash.toString());
            json.put("data", cipher.doFinal(ele.toPrettyString().getBytes()));
            ele = json;
        }
        return this;
    }

    public Element stdUnit(HashMap<String, String> formIn) throws Exception {
        var body = formIn.get("body");
        var chartRgx = Pattern.compile("@chart:[\\d+(,\\d+)*]").matcher(body);
        var attachRgx = Pattern.compile("@file:(\\w+/)+\\w+\\.\\w+").matcher(body);

        if (chartRgx.find()) {
            for (var count = 0; count < chartRgx.groupCount(); count++) {
                var file = Files.readString(Paths.get("chart.html"));
                var html = String.format(file, chartRgx.group(count).split(":")[1]);
                body = body.replace(chartRgx.group(count), html);
            }
        }
        if (attachRgx.find()) {
            for (var count = 0; count < attachRgx.groupCount(); count++) {
                var cid = ipfs.add(new ByteArrayWrapper(attachRgx
                        .group(count).split(":")[1].getBytes()));
                var html = String.format("<a href='%s'>\n  <img src='./file.svg' />\n</a>", cid);
                body = body.replace(attachRgx.group(count), html);
            }
        }
        formIn.forEach((nextK, nextV) -> {
            ele.put(nextK, nextV);
        });
        ele.put("IP", InetAddress.getLocalHost().toString());
        ele.put("date", LocalDate.now()
                .format(DateTimeFormatter.ofPattern("yyyy/MM/dd")).toString());
        return this;
    }

    public Element chat() throws Exception {
        var json = new ObjectMapper().createObjectNode();
        var key = ipfs.key.gen("", Optional.of("rsa"), Optional.of("2048"));

        json.put("key", key.id.toString());
        var cid = ipfs.add(new ByteArrayWrapper(json.toPrettyString().getBytes()));
        var ipns = ipfs.name.publish(cid.get(0).hash, Optional.of(key.id.toString()));
        ele.put("chat", ipns.get(0).toString());
        return this;
    }
}
