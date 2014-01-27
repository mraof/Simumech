package com.mraof.simumech.markov;

import java.io.BufferedReader;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Random;
import java.util.concurrent.locks.ReadWriteLock;
import java.util.concurrent.locks.ReentrantReadWriteLock;

import com.mraof.simumech.Main;
import com.mraof.simumech.Util;

public class MarkovChain 
{
	Random rand = new Random();
	//key is pair of two words
	HashMap<String, ArrayList<String>> wordPairs = new HashMap<String, ArrayList<String>>();
	//key is a single word, used if pair can't be found
	HashMap<String, ArrayList<String>> words = new HashMap<String, ArrayList<String>>();
	HashSet<String> lines = new HashSet<String>();
	HashMap<String, Integer> wordFrequencies = new HashMap<String, Integer>();

	ReadWriteLock lock = new ReentrantReadWriteLock();

	public MarkovChain() 
	{
		try {
			BufferedReader reader = new BufferedReader(new FileReader("lines.txt"));
			String line;
			while((line = reader.readLine()) != null)
			{
				addLine(line);
				if(Main.useCR)
					System.out.printf("Added %d lines\r", lines.size());
			}
			System.out.println();
			reader.close();
		} catch (FileNotFoundException e) {e.printStackTrace();} catch (IOException e) {e.printStackTrace();}
		System.out.printf("Loaded %d lines, %d words, %d word pairs\n", lines.size(), words.size(), wordPairs.size());
	}

	public void addLine(String line)
	{
		lock.writeLock().lock();
		{
			if(!lines.contains(line))
				lines.add(line);
			ArrayList<String> currentWords = Util.split(line);
			String previousWord = "";
			for(int i = 0; i < currentWords.size() - 1; i++)
			{
				String currentWord = Util.selectivelyLowerCase(currentWords.get(i));
				String nextWord = Util.selectivelyLowerCase(currentWords.get(i + 1));
				String pair = previousWord + " " + currentWord;
				ArrayList pairList = wordPairs.get(pair);
				if(pairList == null)
					pairList = new ArrayList<String>();
				pairList.add(nextWord);
				wordPairs.put(pair, pairList);

				ArrayList wordList = words.get(currentWord);
				if(wordList == null)
					wordList = new ArrayList<String>();
				wordList.add(nextWord);
				words.put(currentWord, wordList);
				
				Integer wordFrequency = wordFrequencies.get(currentWord);
				if(wordFrequency == null)
					wordFrequency = 0;
				wordFrequencies.put(currentWord, wordFrequency + 1);
				
				previousWord = currentWord;
//				System.out.println("\"" + currentWords.get(i) + "\",");
			}
		}
		lock.writeLock().unlock();
	}

	public String reply(String inputString)
	{
		ArrayList<String> currentLines;
		ArrayList<String> currentWords = new ArrayList<String>();
		ArrayList<String> index = new ArrayList<String>();
		ArrayDeque<String> sentence = new ArrayDeque<String>();
		String replyString = "";
		boolean done = false;

		if(inputString.isEmpty())
		{
			System.out.println("Empty input string");
			return "";
		}

		String message = inputString;
		currentLines = Util.split(message, ". ");

		for(int i = 0; i < currentLines.size(); i++)
			currentWords.addAll(Util.split(currentLines.get(i)));
		for(int i = 0; i < currentWords.size(); i++)
			currentWords.set(i, Util.selectivelyLowerCase(currentWords.get(i)));
		if(currentWords.isEmpty())
		{
			System.out.println("Input string contained no words");
			return "";
		}
		String previousWord = "";
		for(int i = 0; i < currentWords.size() && sentence.size() < 2; i++)
		{
			String currentWord = currentWords.get(i);
			String pairKey = previousWord + " " + currentWord;
			ArrayList<String> list = wordPairs.get(pairKey);
			if(list != null && rand.nextDouble() > .1)
			{
				String word = list.get(rand.nextInt(list.size()));
//				System.out.println("Adding " + word + " to sentence from pair " + pairKey);
				sentence.add(word);
			}
			else if(rand.nextDouble() > (1 / (currentWords.size() - i + 1) + .2))
			{
				String key = currentWord;
				list = words.get(key);
				if(list != null)
				{
					String word = list.get(rand.nextInt(list.size()));
//					System.out.println("Adding " + word + " to sentence from word " + key);
				}
			}
			previousWord = currentWord;

		}
		if(sentence.isEmpty())
			sentence.add(currentWords.get(0));
		
		for(int size = sentence.size() - 1; size < sentence.size(); )
		{
			size = sentence.size();
			String currentWord = sentence.getLast();
			int wordIndex;
			if((wordIndex = currentWords.indexOf(previousWord)) != -1 && wordIndex < currentWords.size() - 1)
			{
				currentWord = currentWords.get(wordIndex + 1);
				currentWords.remove(wordIndex);
				currentWords.remove(wordIndex);
			}
			String key = previousWord + " " + currentWord;
			ArrayList<String> list = wordPairs.get(key);
			if(list != null && rand.nextDouble() < 4 / (double)sentence.size())
			{
				String word = list.get(rand.nextInt(list.size()));
//				System.out.println("Adding " + word + " to sentence from pair " + key);
				sentence.add(word);
			}
			else /*if(rand.nextDouble() > (1 / (sentence.size() + .5) + .2))*/
			{
				key = currentWord;
				list = words.get(key);
				if(list != null)
				{
					String word = list.get(rand.nextInt(list.size()));
//					System.out.println("Adding " + word + " to sentence from word " + key);
					sentence.add(word);
					int wordFrequency = 1;
					if(words.get(word) != null)
						wordFrequency = words.get(word).size();
//					System.out.println(word + ": " + wordFrequency / (double)sentence.size());
					if(rand.nextDouble() > (wordFrequency / (double)sentence.size()))
					{
						break;
					}
				}
			}
			previousWord = currentWord;

		}

		replyString = sentence.pollFirst();
		if(!replyString.isEmpty())
			replyString = replyString.substring(0, 1).toUpperCase() + replyString.substring(1);
		for(String replyWord : sentence)
			replyString += " " + replyWord;
		return replyString;
	}
	
	

}

