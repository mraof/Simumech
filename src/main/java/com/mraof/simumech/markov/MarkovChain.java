package com.mraof.simumech.markov;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.Random;
import java.util.concurrent.locks.ReadWriteLock;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.regex.Pattern;

import com.mraof.simumech.IntMap;
import com.mraof.simumech.Main;
import com.mraof.simumech.Profiler;
import com.mraof.simumech.Util;

public class MarkovChain {
	Random rand = new Random();
	//key is triple of three words
	HashMap<String, ArrayList<Integer>> wordTriplesNext = new HashMap<String, ArrayList<Integer>>();
	HashMap<String, ArrayList<Integer>> wordTriplesPrevious = new HashMap<String, ArrayList<Integer>>();
	// key is pair of two words
	HashMap<String, ArrayList<Integer>> wordPairsNext = new HashMap<String, ArrayList<Integer>>();
	HashMap<String, ArrayList<Integer>> wordPairsPrevious = new HashMap<String, ArrayList<Integer>>();
	// key is a single word, used if pair can't be found
	HashMap<String, ArrayList<Integer>> wordsNext = new HashMap<String, ArrayList<Integer>>();
	HashMap<String, ArrayList<Integer>> wordsPrevious = new HashMap<String, ArrayList<Integer>>();
	LinkedHashSet<String> lines = new LinkedHashSet<String>();

	IntMap<String, Word> words = new IntMap<String, Word>();
	ReadWriteLock lock = new ReentrantReadWriteLock();

	public MarkovChain() {
		load();
		if (Profiler.instrumentation != null) {
			System.out.println("wordPairsNext: "
					+ Profiler.deepSizeOf(wordPairsNext));
			System.out.println("wordsNext: " + Profiler.deepSizeOf(wordsNext));
			System.out.println("words: " + Profiler.deepSizeOf(words));
			System.out.println("lines: " + Profiler.deepSizeOf(lines));
		}
	}

	public void addLine(String line) {
		lock.writeLock().lock();
		{
			ArrayList<String> splitLines = Util.split(line, ". ", "\n");
			for (String currentLine : splitLines)
			{
				if (!lines.contains(currentLine))
					lines.add(currentLine);
				else
					continue;
				ArrayList<String> currentWords = Util.split(currentLine);
				currentWords.add("");
				String previousWord = "";
				String previousWord2 = "";
				ArrayList<Integer> wordList = null;
				String currentWord;
				String nextWord;
				String nextWord2;
				String pair;
				String triple;
				for (int i = 0; i < currentWords.size() - 1; i++) {
					currentWord = Util.selectivelyLowerCase(currentWords.get(i));
					nextWord = Util.selectivelyLowerCase(currentWords.get(i + 1));
					nextWord2 = i < currentWords.size() - 2 ? Util.selectivelyLowerCase(currentWords.get(i + 2)) : "";

					pair = previousWord.concat(" ").concat(currentWord);
					triple = previousWord2.concat(" ").concat(pair);
					Integer wordIndex = words.lookup(nextWord);
					if (wordIndex == null)
						wordIndex = words.add(new Word(nextWord), nextWord);
					else
						words.get(wordIndex).increment();

					wordList = wordTriplesNext.get(triple);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordTriplesNext.put(triple, wordList);

					wordList = wordPairsNext.get(pair);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordPairsNext.put(pair, wordList);

					wordList = wordsNext.get(currentWord);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordsNext.put(currentWord, wordList);

					wordIndex = words.lookup(previousWord);
					if (wordIndex == null)
						wordIndex = words.add(new Word(previousWord), previousWord);

					pair = currentWord.concat(" ").concat(nextWord);
					triple = pair.concat(" ").concat(nextWord2);

					wordList = wordTriplesNext.get(triple);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordTriplesPrevious.put(triple, wordList);

					wordList = wordPairsNext.get(pair);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordPairsPrevious.put(pair, wordList);

					wordList = wordsPrevious.get(currentWord);
					if (wordList == null)
						wordList = new ArrayList<Integer>();
					wordList.add(wordIndex);
					wordsPrevious.put(currentWord, wordList);

					previousWord = currentWord;
				}
			}
		}
		lock.writeLock().unlock();
	}

	public String reply(String message) {
		return reply(message, "", "");
	}

	public String reply(String inputString, String name, String sender) {
		ArrayList<String> currentLines;
		ArrayList<String> currentWords = new ArrayList<String>();
		ArrayDeque<String> sentence = new ArrayDeque<String>();
		String allSentences = "";
		String replyString = "";
		//int inputLength = inputString.length();

		if (inputString.isEmpty()) {
			System.out.println("Empty input string");
			return "";
		}

		currentLines = Util.split(inputString, ". ");

		currentWords.addAll(Util.split(currentLines.get(currentLines.size() - 1)));
		if(currentLines.size() > 0)
			for (int i = 0; i < currentLines.size() - 1; i++)
				allSentences += this.reply(currentLines.get(i)) + ". ";
		for (int i = 0; i < currentWords.size(); i++)
			currentWords.set(i, Util.selectivelyLowerCase(currentWords.get(i)));
		if (currentWords.isEmpty()) {
			System.out.println("Input string contained no words");
			return "";
		}

		lock.readLock().lock();
		String previousWord = "";
		String bestWord = currentWords.get(0);
		String bestWordPair = " " + currentWords.get(0);
		if(currentWords.size() > 1)
		{
			bestWord = currentWords.get(rand.nextInt(currentWords.size() - 1));
			int pairStart = rand.nextInt(currentWords.size() - 1);
			bestWordPair = (pairStart == 0 ? "" : currentWords.get(pairStart - 1)) + " " + currentWords.get(pairStart);
		}

		for (int i = 0; i < currentWords.size(); i++) {
			String currentWord = currentWords.get(i);
			String pairKey = previousWord + " " + currentWord;
			int bestSize = (wordPairsNext.get(bestWordPair) != null ? wordPairsNext.get(bestWordPair).size() : 0)
				+ (wordPairsPrevious.get(bestWordPair) != null ? wordPairsPrevious.size() : 0);
			if(bestSize == 0) 
				bestWordPair = pairKey;

			bestSize = (wordsNext.get(bestWord) != null ? wordsNext.get(bestWord).size() : 0)
				+ (wordsPrevious.get(bestWord) != null ? wordsPrevious.size() : 0);
			if (bestSize == 0)
				bestWord = currentWord;

			previousWord = currentWord;

		}
		ArrayList<Integer> bestList;
		if ((bestList = wordPairsNext.get(bestWordPair)) != null && bestList.size() > 0 && rand.nextDouble() > .05) {
			if (bestWordPair.charAt(0) != ' ') {
				previousWord = Util.splitFirst(bestWordPair)[1];
				sentence.addAll(Util.split(bestWordPair));
			} else {
				sentence.add(bestWordPair.substring(1));
				previousWord = "";
			}
		} else {
			bestList = wordsNext.get(bestWord);
			if (bestList != null) {
				sentence.add(bestWord);
			}
		}

		if (sentence.isEmpty())
			sentence.add(currentWords.get(0));

		String nextWord = sentence.size() > 1 ? sentence.getLast() : "";
		String nextWord2 = "";

		//Add triple support for repeat prevention?
		HashMap<String, ArrayList<Integer>> wordPairsTemp = new HashMap<String, ArrayList<Integer>>();
		HashMap<String, ArrayList<Integer>> wordsTemp = new HashMap<String, ArrayList<Integer>>();
		for (int size = sentence.size() - 1; size < sentence.size();) 
		{
			size = sentence.size();
			String currentWord = sentence.getFirst();
			String key = currentWord + " " + nextWord;
			ArrayList<Integer> list = wordPairsTemp.get(key);
			if(list == null)
			{
				if(wordPairsPrevious.get(key) != null)
					wordPairsTemp.put(key, new ArrayList<Integer>(wordPairsPrevious.get(key)));
				list = wordPairsTemp.get(key);
			}
			if (list != null && list.size() > 0)
			{
				String triple = key + " " + nextWord2;
				String word;
				if(wordTriplesPrevious.get(triple) != null && wordTriplesPrevious.get(triple).size() > 0 && (rand.nextFloat() > 4F / list.size()))
				{
					list = wordTriplesPrevious.get(triple);
					int index = rand.nextInt(list.size());
					word = words.get(list.get(index)).toString();
				}
				else
				{
					int index = rand.nextInt(list.size());
					word = words.get(list.get(index)).toString();
					list.remove(index);
				}
				if(!word.isEmpty())
				{
					sentence.addFirst(word);
					//System.out.println("\"" + word + " <-- \"" + key + "\"");
				}
			}
			else
			{
				key = currentWord;
				list = wordsTemp.get(key);
				if(sentence.size() / (double) currentWords.size() > rand.nextDouble() && list == null)
				{
					if(wordsPrevious.get(key) != null)
						wordsTemp.put(key, new ArrayList<Integer>(wordsPrevious.get(key)));
					list = wordsTemp.get(key);
				}
				if (list != null && list.size() > 0)
				{
					int index = rand.nextInt(list.size());
					String word = words.get(list.get(index)).toString();
					list.remove(index);
					if(!word.isEmpty())
					{
						sentence.addFirst(word);
						//System.out.println("\"" + word + " <-- \"" + key + "\"");
					}
				}
			}
			nextWord2 = nextWord;
			nextWord = currentWord;
			//System.out.println(sentence);

		}

		String previousWord2 = "";
		if(sentence.size() > 1)
		{
			String[] sentenceArray = sentence.toArray(new String[sentence.size()]);
			previousWord = sentenceArray[sentence.size() - 2]; //Get second to last word
			if(sentence.size() > 2)
				previousWord2 = sentenceArray[sentence.size() - 3];
		}
		wordPairsTemp = new HashMap<String, ArrayList<Integer>>();
		wordsTemp = new HashMap<String, ArrayList<Integer>>();

		for (int size = sentence.size() - 1; size < sentence.size();) {
			size = sentence.size();
			String currentWord = sentence.getLast();
			String key = previousWord + " " + currentWord;
			ArrayList<Integer> list = wordPairsTemp.get(key);
			if(list == null)
			{
				if(wordPairsNext.get(key) != null)
					wordPairsTemp.put(key, new ArrayList<Integer>(wordPairsNext.get(key)));
				list = wordPairsTemp.get(key);
			}
			if (list != null && list.size() > 0) {
				int index = rand.nextInt(list.size());
				String word = words.get(list.get(index)).toString();
				list.remove(index);
				//System.out.println("\"" + key + "\" --> \"" + word + "\"");
				if(!word.isEmpty())
				{
					sentence.add(word);
				}
			} else {
				key = currentWord;
				list = wordsTemp.get(key);
				if(list == null)
				{
					if(wordsNext.get(key) != null)
						wordsTemp.put(key, new ArrayList<Integer>(wordsNext.get(key)));
					list = wordsTemp.get(key);
				}
				if (list != null && list.size() > 0)
				{

					String triple = previousWord2 + " " + key;
					String word;
					if(wordTriplesNext.get(triple) != null && wordTriplesNext.get(triple).size() > 0 && (rand.nextFloat() > 4F / list.size()))
					{
						list = wordTriplesNext.get(triple);
						int index = rand.nextInt(list.size());
						word = words.get(list.get(index)).toString();
					}
					else
					{
						int index = rand.nextInt(list.size());
						word = words.get(list.get(index)).toString();
						list.remove(index);
						//System.out.println("\"" + key + "\" --> \"" + word + "\"");
					}
					if(!word.isEmpty());
					{
						sentence.add(word);
					}
				}
			}
			previousWord2 = previousWord;
			previousWord = currentWord;

		}
		lock.readLock().unlock();

		do
			replyString = sentence.pollFirst();
		while (replyString.isEmpty());

		if (!replyString.isEmpty() && !replyString.startsWith("http"))
			replyString = replyString.substring(0, 1).toUpperCase()
				+ replyString.substring(1);
		if (replyString.equalsIgnoreCase(name) && !sender.isEmpty())
			replyString = sender;
		for (String replyWord : sentence)
			if (!replyWord.isEmpty())
				replyString += " " + replyWord;
		return allSentences + replyString;
	}

	public String randomSentence() {
		String firstWord;
		ArrayList<Integer> list;
		do {
			firstWord = words.get(rand.nextInt(words.size())).toString();
			list = wordsNext.get(firstWord);
		} while (list == null);
		String secondWord = words.get(list.get(rand.nextInt(list.size())))
			.toString();
		return reply(firstWord + " " + secondWord);
	}

	public String command(String command) {
		ArrayList<String> parts = Util.split(command);
		switch (parts.get(0).toUpperCase()) {
			case "STATS":
				return "Word triples next: " + wordTriplesNext.size() + ", Word pairs next: " + wordPairsNext.size()
					+ ", words next: " + wordsNext.size() + ", word triples previous: " + wordTriplesPrevious.size() + ", word pairs previous: " 
					+ wordPairsPrevious.size() + ", words previous: " + wordsPrevious.size() + ", words: "
					+ words.size() + ", lines: " + lines.size();
			case "LINES":
				if (parts.size() > 1) {
					String word = parts.get(1).toLowerCase();
					String matchingLines = "";
					int matches = 0;
					for (String line : lines)
						if (line.toLowerCase().matches(
									".*\\b(" + Pattern.quote(word) + ")\\b.*")) {
							matchingLines += "\n" + line;
							matches++;
									}
					if (matches < 10 || parts.size() > 2
							&& parts.get(2).equalsIgnoreCase("YES"))
						return "Found \"" + word + "\" " + matches + " times"
							+ matchingLines;
				}
			case "KNOWN":
				if (parts.size() > 1) {
					String word = parts.get(1).toLowerCase();
					int matches = 0;
					for (String line : lines)
						if (line.toLowerCase().matches(
									".*\\b(" + Pattern.quote(word) + ")\\b.*"))
							matches++;
					return "Found \"" + word + "\" " + matches + " times";
				} else
					return "Invalid syntax, should be "
						+ parts.get(0).toUpperCase() + " <word>";
			case "WORDSTATS":
				if (parts.size() > 1) {
					String wordString = Util.selectivelyLowerCase(parts.get(1));
					if (words.lookup(wordString) != null) {
						Word word = words.get(words.lookup(wordString));
						int empty = words.lookup("");
						String nextWordString = "";
						int nextWordCount = 0;
						String previousWordString = "";
						int previousWordCount = 0;
						ArrayList<Integer[]> nextWords = new ArrayList<Integer[]>();
						ArrayList<Integer[]> previousWords = new ArrayList<Integer[]>();
						if (wordsNext.get(wordString.toLowerCase()) != null)
							for (Integer index : wordsNext.get(wordString.toLowerCase())) 
							{
								if(index == empty)
									continue;
								boolean notFound = true;
								for (int i = 0; i < nextWords.size(); i++)
									if (nextWords.get(i)[0] == index) 
									{
										nextWords.get(i)[1]++;
										if (nextWords.get(i)[1] > nextWordCount) 
										{
											nextWordCount = nextWords.get(i)[1];
											nextWordString = words.get(nextWords.get(i)[0]).toString();
										}
										notFound = false;
										break;
									}
								if (notFound )
									nextWords.add(new Integer[] { index, 1 });
							}

						if (wordsPrevious.get(wordString.toLowerCase()) != null)
							for (Integer index : wordsPrevious.get(wordString.toLowerCase())) 
							{
								if(index == empty)
									continue;
								boolean notFound = true;
								for (int i = 0; i < previousWords.size(); i++)
									if (previousWords.get(i)[0] == index) 
									{
										previousWords.get(i)[1]++;
										if (previousWords.get(i)[1] > previousWordCount) 
										{
											previousWordCount = previousWords.get(i)[1];
											previousWordString = words.get(previousWords.get(i)[0]).toString();
										}
										notFound = false;
										break;
									}
								if (notFound)
									previousWords.add(new Integer[] { index, 1 });
							}

						return "\"" + wordString + "\" has a count of "
							+ word.getCount() + ", an index of "
							+ words.lookup(wordString)
							+ ", with a most common next word of \""
							+ nextWordString + "\" (" + nextWordCount
							+ " times) and a most common previous word of \""
							+ previousWordString + "\" (" + previousWordCount + ")";
					} else
						return wordString + " is not known";
				}
			case "WORDID":
				if (parts.size() > 1) {
					try {
						Integer index = Integer.decode(parts.get(1));
						return words.get(index).toString();
					} catch (ArrayIndexOutOfBoundsException e) {
						return "ID out of bounds";
					} catch (NumberFormatException e) {
						return "\"" + parts.get(1) + "\" is not a number";
					}
				}
			case "SENTENCE":
				return randomSentence();
		}

		return "";
	}

	public void load() {
		long startTime = System.currentTimeMillis();
		try {
			BufferedReader reader = new BufferedReader(new FileReader(
						"lines.txt"));
			String line;
			while ((line = reader.readLine()) != null) {
				addLine(line);
				if (Main.useCR)
					System.out.printf("Added %d lines\r", lines.size());
			}
			System.out.println();
			reader.close();
		} catch (FileNotFoundException e) {
			e.printStackTrace();
		} catch (IOException e) {
			e.printStackTrace();
		}
		long endTime = System.currentTimeMillis();

		System.out
			.printf("Loaded %d lines, %d words, %d word pairs in %d milliseconds\n",
					lines.size(), wordsNext.size(), wordPairsNext.size(),
					endTime - startTime);
	}

	public void save() {
		// try {
		// Files.copy(new File("lines.txt").toPath(), new
		// File("lines.bak.txt").toPath());
		// } catch (IOException e)
		// {System.err.println("Unable to backup file before saving");}
		lock.readLock().lock();
		try {
			BufferedWriter writer = new BufferedWriter(new FileWriter(
						"lines.txt"));
			for (String line : lines) {
				writer.write(line);
				writer.newLine();
			}
			writer.close();
		} catch (IOException e) {
			e.printStackTrace();
		}
		lock.readLock().unlock();
	}

}
