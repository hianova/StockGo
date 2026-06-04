package hianova.stockgo;

import java.nio.file.Paths;
import java.util.regex.Pattern;
import java.util.stream.IntStream;

import org.jsoup.Jsoup;
import org.jsoup.nodes.Element;

import com.fasterxml.jackson.databind.ObjectMapper;

public class Lib {

  public String URLToName(String URLIn) {
    var removeRgx = Pattern.compile("(\\.\\w+|@num|@date|\\?.+)");
    var tmp = removeRgx.matcher(URLIn).replaceAll("").split("/");
    var out = tmp[tmp.length - 2] + "_" + tmp[tmp.length - 1];
    return out;
  }

  public Element cleanHTML(String pathIn) {
    var page = Jsoup.parse(pathIn);
    var out = new Element("html");

    page.select("tr").forEach(nextRow -> {
      if (nextRow.children().select("table").isEmpty()
          & nextRow.parent().select("table").isEmpty()) {
        var line = new Element("tr");
        nextRow.children().forEach(nextCell -> {
          nextCell.select("br").remove();
          line.appendChild(nextCell);
          if (nextCell.hasAttr("colspan")) {
            IntStream.range(1, Integer.parseInt(nextCell.attr("colspan")))
                .forEach(next -> line.appendChild(nextCell.clone().removeAttr("colspan")));
          }
        });
        out.appendChild(line);
      }
    });
    IntStream.range(0, out.childNodeSize()).forEach(nextRow -> {
      var row = out.child(nextRow);
      IntStream.range(0, row.childrenSize()).forEach(nextCell -> {
        var cell = row.children().get(nextCell);
        if (cell.hasAttr("rowspan")) {
          IntStream.range(1, Integer.parseInt(cell.attr("rowspan")))
              .forEach(next -> out.child(nextRow + next)
                  .insertChildren(nextCell, cell.clone().removeAttr("rowspan")));
        }
      });
    });
    out.appendChild(page.select("tag").first());
    return out;
  }

  public boolean isJSON(String JSONIn) {
     try {
      new ObjectMapper().readTree(JSONIn);
      return true;
    } catch (Exception e) {
      return false;
    }
   }

  public boolean isHTML(String HTMLIn) {
    return Jsoup.parse(HTMLIn).childNodeSize() > 0;
  }

  public String tag(String tagIn) throws Exception {
    var file = Paths.get( "parse_rule.json").toFile();
    var out = new ObjectMapper().readTree(file).at("/" + tagIn).textValue();

    if (out == null) {
      var tmp = tagIn.split("/");
      if (tmp.length >= 3) {
        if (tmp[1].equals("codec") & tmp[2].equals("UTF-8")) {
          out = "UTF-8";
        } else if (tmp[1].equals("head")) {
          if (tmp[2].equals("JSON")) {
            out = "fields";
          } else if (tmp[2].equals("HTML")) {
            out = "tr:eq(0)";
          }
        } else if (tmp[1].equals("body")) {
          if (tmp[2].equals("JSON")) {
            out = "data";
          } else if (tmp[2].equals("HTML")) {
            out = "tr:gt(0)";
          }
        }
      }
    }
    return out;
  }

}
