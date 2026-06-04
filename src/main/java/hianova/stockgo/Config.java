package hianova.stockgo;

import java.nio.file.Files;
import java.nio.file.Paths;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.concurrent.TimeUnit;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.google.common.cache.Cache;
import com.google.common.cache.CacheBuilder;

public class Config {

  protected final Lib LIB;
  protected final ArrayList<String> CONFIG_TITLE, CONFIG_URL, CONFIG_FOLDER, CONFIG_LABEL, CONFIG_STATUS;
  protected final Pattern NUM_RGX, DATE_RGX;
  protected final DateTimeFormatter UNI_DATE;
  private final Cache<String, ArrayList<String>> CACHE;

  public Config() throws Exception {
    LIB = new Lib();
    CONFIG_TITLE = new ArrayList<>();
    CONFIG_URL = new ArrayList<>();
    CONFIG_FOLDER = new ArrayList<>();
    CONFIG_LABEL = new ArrayList<>();
    CONFIG_STATUS = new ArrayList<>();
    NUM_RGX = Pattern.compile("@num");
    DATE_RGX = Pattern.compile("@date");
    UNI_DATE = DateTimeFormatter.ofPattern("yyyyMMdd");
    CACHE = CacheBuilder.newBuilder()
        .maximumSize(1000)
        .expireAfterWrite(20, TimeUnit.MINUTES)
        .build();

    new ObjectMapper().readTree(Paths.get("downloads", "config.json").toFile()).fields()
        .forEachRemaining(next -> {
          CONFIG_TITLE.add(next.getKey());
          CONFIG_URL.add(next.getValue().get("URL").textValue());
          CONFIG_FOLDER.add(next.getValue().get("folder").textValue());
          CONFIG_LABEL.add(next.getValue().get("tag").textValue());
          CONFIG_STATUS.add(next.getValue().get("status").textValue());
        });
  }

  public ArrayList<String> streamDate(String urlIn, String stEdIn) {
    if (!DATE_RGX.matcher(urlIn).find()) {
      return new ArrayList<>(List.of(""));
    }
    LocalDate[] stEd;
    var out = new ArrayList<String>();
    var num = CONFIG_URL.indexOf(urlIn);
    var dcRgx = Pattern.compile("yyyy");
    var stEdRgx = Pattern.compile("\\d+~\\d+");
    var label = Arrays.stream(CONFIG_LABEL.get(num).split(",")).dropWhile(
        next -> DATE_RGX.matcher(next).find()).collect(Collectors.joining()).split(":");

    if (stEdRgx.matcher(stEdIn).find()) {
      var tmp = stEdIn.split("~");
      stEd = new LocalDate[] {
          LocalDate.parse(tmp[0], UNI_DATE), LocalDate.parse(tmp[1], UNI_DATE) };
    } else {
      stEd = new LocalDate[] {
          LocalDate.parse(CONFIG_STATUS.get(num), UNI_DATE), LocalDate.now() };
    }
    if (!dcRgx.matcher(label[1]).find()) {
      stEd[0] = stEd[0].plusYears(1911);
      stEd[1] = stEd[1].plusYears(1911);
    }
    while (stEd[0].isBefore(stEd[1])) {
      out.add(stEd[0].format(UNI_DATE));
      if (label[2].equals("M")) {
        stEd[0] = stEd[0].plusMonths(1);
      } else if (label[2].equals("W")) {
        stEd[0] = stEd[0].plusWeeks(1);
      } else if (label[2].equals("D")) {
        stEd[0] = stEd[0].plusDays(1);
      }
    }
    return out;
  }

  public ArrayList<String> streamNum(String URLIn, String numIn) throws Exception {
    if (!NUM_RGX.matcher(URLIn).find()) {
      return new ArrayList<>(List.of(""));
    }
    ArrayList<String> out;
    var numInRgx = Pattern.compile("\\w+(\\.\\w+)+");

    if (numInRgx.matcher(numIn).find()) {
      out = new ArrayList<>(List.of(numIn.split("\\.")));
    } else {
      var label = Arrays.stream(CONFIG_LABEL.get(CONFIG_URL.indexOf(URLIn)).split(",")).dropWhile(
          next -> NUM_RGX.matcher(next).find()).collect(Collectors.joining()).split(":");
      if (label[0].equals("stock")) {
        out = stockNum(label.length > 1 ? label[1] : "");
      } else if (label[0].equals("ETF")) {
        out = ETFNum();
      } else {
        out = new ArrayList<>(List.of(""));
      }
    }
    return out;
  }

  public ArrayList<String> listConfig() {
    var out = new ArrayList<>(List.of("number", "title", "URL", "folder", "label", "status", "\n"));

    IntStream.range(0, CONFIG_TITLE.size()).forEach(next -> out.addAll(
        List.of(String.valueOf(next), CONFIG_TITLE.get(next), CONFIG_URL.get(next),
            CONFIG_FOLDER.get(next), CONFIG_LABEL.get(next), CONFIG_STATUS.get(next) + "\n")));
    return out;
  }

  public void syncConfig() throws Exception {
    Files.writeString(Paths.get("downloads", "config.txt"), toString());
  }

  public String toOrigin(String dateIn, String URLIn) {
    if (dateIn.isBlank()) {
      return dateIn;
    }
    var label = Arrays.stream(CONFIG_LABEL.get(CONFIG_URL.indexOf(URLIn)).split(","))
        .dropWhile(part -> NUM_RGX.matcher(part).find())
        .collect(Collectors.joining())
        .split(":")[1];
    var out = LocalDate.parse(dateIn, UNI_DATE).format(DateTimeFormatter.ofPattern(label));
    return out;
  }

  @Override
  public String toString() {
    String out;
    var json = new ObjectMapper().createObjectNode();

    CONFIG_TITLE.forEach(next -> {
      var num = CONFIG_TITLE.indexOf(next);
      var node = json.putObject(next);
      node.put("URL", CONFIG_URL.get(num));
      node.put("folder", CONFIG_FOLDER.get(num));
      node.put("label", CONFIG_LABEL.get(num));
      node.put("status", CONFIG_STATUS.get(num));
    });
    out = json.toPrettyString();
    return out;
  }

  public HashMap<String, String> relay(String nameIn) throws Exception {
    var out = new HashMap<String, String>();
    var json = new ObjectMapper().readTree(Paths.get("downloads", "relay.json").toFile());
    var exist = json.has(nameIn);

    out.put("URL", exist ? json.at(nameIn + "/URL").asText() : "");
    out.put("request", exist ? json.at(nameIn + "/request").asText() : "");
    out.put("date", exist ? json.at(nameIn + "/date").asText() : "");
    out.put("num", exist ? json.at(nameIn + "/num").asText() : "");
    return out;
  }

  public ArrayList<String> stockNum(String nameIn) throws Exception {
    var out = new ArrayList<String>();
    var list = new ArrayList<String>();
    var rgx = Pattern.compile("^[0-9]{4}　");

    if (nameIn == "listed" | nameIn == "上市") {
      list.add("上市");
    } else if (nameIn == "OTC" | nameIn == "上櫃") {
      list.add("上櫃");
    } else {
      list.addAll(List.of("上市", "上櫃"));
    }
    list.forEach(nextList -> {
      try {
        var tmp = new Parse(Paths.get("downloads", nextList + "證券代號", "isin_C_public.txt").toString(),
            new ArrayList<>(List.of("有價證券代號及名稱"))).data();
        tmp[0].forEach(
            next -> {
              if (rgx.matcher(next).find()) {
                out.add(next.split("　")[0]);
              }
            });
      } catch (Exception e) {
        System.out.println("number not exist");
      }
    });
    return out;
  }

  public ArrayList<String> ETFNum() throws Exception {
    var out = new ArrayList<String>();
    var rgx = Pattern.compile("^T[0-9]+\\w");
    var tmp = new Parse(Paths.get("downloads" + "基金＿國際證券代號", "isin_C_public.txt").toString(),
        new ArrayList<>(List.of("有價證券代號及名稱"))).data();
        
    tmp[0].forEach(next -> {
      if (rgx.matcher(next).find()) {
        out.add(next.split("　")[0]);
      }
    });
    return out;
  }

  public void setCache(String nameIn, ArrayList<String> objIn) {
    CACHE.put(nameIn, objIn);
  }

  public ArrayList<String> getCache(String nameIn) {
    return CACHE.getIfPresent(nameIn);
  }

}
