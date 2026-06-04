package hianova.stockgo;

import java.io.StringReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

import javax.script.ScriptEngineManager;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.opencsv.CSVReader;

public class Parse {

  private final ArrayList<String> HD, BD, OPT;
  private final ArrayList<Integer> REQ;
  private final ArrayList<Pattern> TAG;
  private boolean checkTag, checkOpt;

  public Parse(String pathIn, ArrayList<String> reqIn) throws Exception {
    var lib = new Lib();
    var optRgx = Pattern.compile("!");
    var tagRgx = Pattern.compile("#");
    var file = Files.readString(Paths.get(pathIn));
    OPT = new ArrayList<>();
    HD = new ArrayList<>();
    BD = new ArrayList<>();
    REQ = new ArrayList<>();
    TAG = new ArrayList<>();

    if (lib.isHTML(file)) {
      var html = lib.cleanHTML(file);
      var tag = html.select("tag").text();
      var hdTmp = html.select(lib.tag(tag + "/head/HTML"));
      var bdTmp = html.select(lib.tag(tag + "/body/HTML"));
      HD.addAll(hdTmp.first().children().eachText());
      bdTmp.forEach(next -> BD.addAll(next.children().eachText()));
    } else if (lib.isJSON(file)) {
      var json = new ObjectMapper().readTree(file);
      var tag = json.at("tag").textValue();
      var hdTmp = json.at(lib.tag(tag + "/head/JSON"));
      var bdTmp = json.at(lib.tag(tag + "/body/JSON"));
      hdTmp.forEach(node -> HD.add(node.textValue()));
      bdTmp.forEach(row -> row.forEach(cell -> BD.add(cell.textValue())));
    } else {
      try (var csv = new CSVReader(new StringReader(file))) {
        var iter = csv.iterator();
        for (var tmp : iter.next()) {
          HD.add(tmp);
        }
        for (var row = iter.next(); iter.hasNext(); row = iter.next()) {
          for (var cell : row) {
            BD.add(cell);
          }
        }
      }
    }
    reqIn.forEach(next -> {
      var reqTmp = next;
      if (optRgx.matcher(reqTmp).find()) {
        var tmp = reqTmp.split("!");
        OPT.add(tmp[0]);
        reqTmp = tmp[1];
        checkOpt = true;
      } else {
        OPT.add("");
      }
      if (tagRgx.matcher(reqTmp).find()) {
        var tmp = reqTmp.split("#");
        var rgx = tmp[1];
        reqTmp = tmp[0];
        checkTag = true;
        for (var count = 2; count < tmp.length; count++) {
          rgx = String.format("%s|%s", rgx, tmp[count]);
        }
        TAG.add(Pattern.compile(rgx));
      } else {
        TAG.add(Pattern.compile(""));
      }
      REQ.add(HD.indexOf(reqTmp));
    });
  }

  public ArrayList<String>[] data() {
    ArrayList<String>[] out = new ArrayList[REQ.size()];

    Arrays.setAll(out, i -> new ArrayList<>());
    for (var row = 0; row < (BD.size() / HD.size()); row++) {
      var pass = !checkTag;
      var line = new ArrayList<String>();
      for (var cell = 0; cell < HD.size(); cell++) {
        var tmp = BD.stream().skip(row * HD.size() + REQ.get(cell)).limit(1).findFirst().orElse("null");
        line.add(tmp.isBlank() ? "null" : tmp);
        if (checkTag & !pass & TAG.get(cell).matcher(tmp).find()) {
          pass = true;
        }
      }
      if (pass) {
        IntStream.range(0, out.length).forEach(
            next -> out[next].add(line.get(next)));
      }
    }
    if (checkOpt) {
      IntStream.range(0, OPT.size()).forEach(next -> {
        if (out[next].isEmpty()) {
          return;
        }
        try {
          var script = new ScriptEngineManager().getEngineByName("javascipt");
          var tmp = out[next].stream().map(Integer::parseInt).collect(Collectors.toList());
          script.put("x", tmp);
          script.put("y", "");
          script.eval(String.format("y= x.map(num =>%s);", OPT.get(next)));
          out[next] = (ArrayList<String>) script.get("y");
        } catch (Exception e) {
          System.out.println("operator cant apply");
        }
      });
    }
    return out;
  }
}